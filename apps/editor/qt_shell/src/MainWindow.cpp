#include "MainWindow.h"

#include <algorithm>
#include <cmath>
#include <QAction>
#include <QApplication>
#include <QDockWidget>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QHeaderView>
#include <QFontDatabase>
#include <QFormLayout>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonValue>
#include <QLabel>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPainter>
#include <QPainterPath>
#include <QProcess>
#include <QSaveFile>
#include <QStatusBar>
#include <QTreeWidgetItemIterator>
#include <QTreeWidgetItem>
#include <QUuid>
#include <QVBoxLayout>
#include <QMouseEvent>

CanvasWidget::CanvasWidget(QWidget* parent) : QWidget(parent) {
  setMinimumSize(720, 520);
  setAutoFillBackground(true);
}

void CanvasWidget::setScene(const SceneDocumentData& scene) {
  scene_ = scene;
  update();
}

void CanvasWidget::setSelectedNode(const SceneNodeData& node) {
  selectedNode_ = node;
  update();
}

void CanvasWidget::paintEvent(QPaintEvent* event) {
  Q_UNUSED(event);

  QPainter painter(this);
  painter.setRenderHint(QPainter::Antialiasing, true);
  painter.fillRect(rect(), QColor("#ddd7ca"));

  const QRectF canvasRect = canvasRectForWidget();
  painter.fillRect(canvasRect, scene_.background.isValid() ? scene_.background : QColor("#f5f1e8"));
  painter.setPen(QPen(QColor("#8c8174"), 2.0));
  painter.drawRect(canvasRect);

  painter.setPen(QColor("#2f241f"));
  QFont titleFont = painter.font();
  titleFont.setPointSize(18);
  titleFont.setBold(true);
  painter.setFont(titleFont);
  painter.drawText(QRectF(56.0, 56.0, canvasRect.width() - 32.0, 36.0),
                   QString("Canvas Placeholder: %1").arg(scene_.name));

  QFont bodyFont = painter.font();
  bodyFont.setPointSize(12);
  bodyFont.setBold(false);
  painter.setFont(bodyFont);

  const QString summary = QString("Scene: %1 x %2\nSelected: %3 [%4]\nRender items: %5")
                              .arg(scene_.width, 0, 'f', 0)
                              .arg(scene_.height, 0, 'f', 0)
                              .arg(selectedNode_.name.isEmpty() ? QString("None") : selectedNode_.name)
                              .arg(selectedNode_.type.isEmpty() ? QString("-") : selectedNode_.type)
                              .arg(scene_.renderItems.size());

  painter.drawText(QRectF(56.0, 108.0, canvasRect.width() - 64.0, 72.0), summary);

  const QPointF dragOffset = activeDragWidgetOffset();

  for (const auto& item : scene_.renderItems) {
    const QColor fill = item.fill.isValid() ? item.fill : QColor("#c8bfb1");
    painter.setOpacity(item.opacity);
    const bool dragSelectedItem = dragActive_ && item.nodeId == dragNodeId_;
    const bool simplifyForInteraction = dragActive_;

    if (dragSelectedItem) {
      painter.save();
      painter.translate(dragOffset);
    }

    auto shadowOffsetForItem = [&](const SceneCanvasItemData& shadowItem) -> QPointF {
      if (!shadowItem.hasShadow) {
        return QPointF(0.0, 0.0);
      }
      const QPointF base = mapScenePoint(ScenePointData{0.0, 0.0}, canvasRect);
      const QPointF offset = mapScenePoint(
          ScenePointData{shadowItem.shadow.offsetX, shadowItem.shadow.offsetY}, canvasRect);
      return offset - base;
    };

    auto drawApproxBlur = [&](auto drawFn) {
      if (simplifyForInteraction || item.blurRadius <= 0.0) {
        return;
      }

      const int passes = std::clamp(static_cast<int>(std::ceil(item.blurRadius / 2.0)), 1, 6);
      QColor blurColor = fill;
      blurColor.setAlphaF(0.08);

      for (int dy = -passes; dy <= passes; ++dy) {
        for (int dx = -passes; dx <= passes; ++dx) {
          if (dx == 0 && dy == 0) {
            continue;
          }
          painter.save();
          painter.translate(dx, dy);
          drawFn(blurColor);
          painter.restore();
        }
      }
    };

    auto drawApproxShadow = [&](auto drawFn) {
      if (simplifyForInteraction || !item.hasShadow) {
        return;
      }

      const QPointF offset = shadowOffsetForItem(item);
      const int passes =
          std::clamp(static_cast<int>(std::ceil(item.shadow.blurRadius / 4.0)), 1, 6);
      QColor shadowColor = item.shadow.color;
      shadowColor.setAlphaF(std::max(0.04, shadowColor.alphaF() * 0.08));

      for (int dy = -passes; dy <= passes; ++dy) {
        for (int dx = -passes; dx <= passes; ++dx) {
          painter.save();
          painter.translate(offset.x() + dx, offset.y() + dy);
          drawFn(shadowColor);
          painter.restore();
        }
      }
    };

    if (item.kind == "Rectangle" && item.hasBounds) {
      drawApproxBlur([&](const QColor& blurColor) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(blurColor);
        painter.drawRoundedRect(mapSceneRect(item.bounds, canvasRect), item.cornerRadius,
                                item.cornerRadius);
      });
      drawApproxShadow([&](const QColor& shadowColor) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(shadowColor);
        painter.drawRoundedRect(mapSceneRect(item.bounds, canvasRect), item.cornerRadius,
                                item.cornerRadius);
      });
      painter.setPen(Qt::NoPen);
      painter.setBrush(fill);
      painter.drawRoundedRect(mapSceneRect(item.bounds, canvasRect), item.cornerRadius,
                              item.cornerRadius);
    } else if (item.kind == "Ellipse" && item.hasBounds) {
      drawApproxBlur([&](const QColor& blurColor) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(blurColor);
        painter.drawEllipse(mapSceneRect(item.bounds, canvasRect));
      });
      drawApproxShadow([&](const QColor& shadowColor) {
        painter.setPen(Qt::NoPen);
        painter.setBrush(shadowColor);
        painter.drawEllipse(mapSceneRect(item.bounds, canvasRect));
      });
      painter.setPen(Qt::NoPen);
      painter.setBrush(fill);
      painter.drawEllipse(mapSceneRect(item.bounds, canvasRect));
    } else if (item.kind == "Path" && !item.points.isEmpty()) {
      drawApproxShadow([&](const QColor& shadowColor) {
        QPainterPath shadowPath;
        shadowPath.moveTo(mapScenePoint(item.points.first(), canvasRect));
        for (qsizetype index = 1; index < item.points.size(); ++index) {
          shadowPath.lineTo(mapScenePoint(item.points.at(index), canvasRect));
        }
        if (item.closed) {
          shadowPath.closeSubpath();
        }
        painter.setPen(Qt::NoPen);
        painter.setBrush(shadowColor);
        painter.drawPath(shadowPath);
      });

      drawApproxBlur([&](const QColor& blurColor) {
        QPainterPath blurPath;
        blurPath.moveTo(mapScenePoint(item.points.first(), canvasRect));
        for (qsizetype index = 1; index < item.points.size(); ++index) {
          blurPath.lineTo(mapScenePoint(item.points.at(index), canvasRect));
        }
        if (item.closed) {
          blurPath.closeSubpath();
        }
        painter.setPen(Qt::NoPen);
        painter.setBrush(blurColor);
        painter.drawPath(blurPath);
      });

      QPainterPath path;
      path.moveTo(mapScenePoint(item.points.first(), canvasRect));
      for (qsizetype index = 1; index < item.points.size(); ++index) {
        path.lineTo(mapScenePoint(item.points.at(index), canvasRect));
      }
      if (item.closed) {
        path.closeSubpath();
      }
      painter.setPen(item.closed ? Qt::NoPen : QPen(fill, 2.0));
      painter.setBrush(item.closed ? fill : Qt::NoBrush);
      painter.drawPath(path);
    } else if (item.kind == "Text" && item.hasOrigin) {
      QFont textFont = painter.font();
      textFont.setPixelSize(static_cast<int>(std::round(item.fontSize <= 0.0 ? 18.0 : item.fontSize)));
      const QString requestedFamily = item.fontFamily.trimmed();
      const auto families = QFontDatabase::families();
      if (!requestedFamily.isEmpty() && families.contains(requestedFamily)) {
        textFont.setFamily(requestedFamily);
      } else if (families.contains("Arial")) {
        textFont.setFamily("Arial");
      } else if (families.contains("Helvetica")) {
        textFont.setFamily("Helvetica");
      }
      painter.setFont(textFont);
      drawApproxBlur([&](const QColor& blurColor) {
        painter.setPen(blurColor);
        painter.drawText(mapScenePoint(item.origin, canvasRect), item.text);
      });
      drawApproxShadow([&](const QColor& shadowColor) {
        painter.setPen(shadowColor);
        painter.drawText(mapScenePoint(item.origin, canvasRect), item.text);
      });
      painter.setPen(fill);
      painter.drawText(mapScenePoint(item.origin, canvasRect), item.text);
    } else if (item.kind == "ImageLayer" && item.hasBounds) {
      const QRectF imageRect = mapSceneRect(item.bounds, canvasRect);
      painter.setPen(QPen(QColor("#6e5a4d"), 1.5, Qt::DashLine));
      painter.setBrush(QColor("#efe7da"));
      painter.drawRect(imageRect);
      painter.setPen(QColor("#6e5a4d"));
      painter.drawText(imageRect.adjusted(8.0, 8.0, -8.0, -8.0),
                       Qt::AlignLeft | Qt::AlignTop | Qt::TextWordWrap,
                       item.imageRef.isEmpty() ? QString("ImageLayer") : item.imageRef);
    }

    if (dragSelectedItem) {
      painter.restore();
    }
  }

  if (selectedNode_.hasBounds) {
    QRectF selectedRect = mapSceneRect(selectedNode_.bounds, canvasRect);
    if (dragActive_ && selectedNode_.id == dragNodeId_) {
      selectedRect.translate(dragOffset);
    }
    const QRectF outlineRect = selectedRect.adjusted(-6.0, -6.0, 6.0, 6.0);
    painter.setOpacity(1.0);
    painter.setPen(QPen(QColor("#d55a2a"), 1.75));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(outlineRect);

    const qreal handleSize = 8.0;
    const QColor handleColor("#fff8ee");
    const QColor handleStroke("#d55a2a");
    const QList<QPointF> handles = {
        outlineRect.topLeft(),
        outlineRect.topRight(),
        outlineRect.bottomLeft(),
        outlineRect.bottomRight(),
    };

    painter.setPen(QPen(handleStroke, 1.5));
    painter.setBrush(handleColor);
    for (const auto& handleCenter : handles) {
      painter.drawRect(QRectF(handleCenter.x() - handleSize * 0.5,
                              handleCenter.y() - handleSize * 0.5, handleSize, handleSize));
    }
  }

  painter.setOpacity(1.0);
}

QRectF CanvasWidget::canvasRectForWidget() const {
  const QRectF outer(40.0, 40.0, width() - 80.0, height() - 80.0);
  if (scene_.width <= 0.0 || scene_.height <= 0.0) {
    return outer;
  }

  const double scale =
      std::min(outer.width() / scene_.width, outer.height() / scene_.height);
  const double scaledWidth = scene_.width * scale;
  const double scaledHeight = scene_.height * scale;
  const double x = outer.x() + (outer.width() - scaledWidth) * 0.5;
  const double y = outer.y() + (outer.height() - scaledHeight) * 0.5;
  return QRectF(x, y, scaledWidth, scaledHeight);
}

QPointF CanvasWidget::mapScenePoint(const ScenePointData& point, const QRectF& canvasRect) const {
  if (scene_.width <= 0.0 || scene_.height <= 0.0) {
    return QPointF(canvasRect.x() + point.x, canvasRect.y() + point.y);
  }

  const double scaleX = canvasRect.width() / scene_.width;
  const double scaleY = canvasRect.height() / scene_.height;
  return QPointF(canvasRect.x() + point.x * scaleX, canvasRect.y() + point.y * scaleY);
}

QRectF CanvasWidget::mapSceneRect(const SceneRectData& rect, const QRectF& canvasRect) const {
  if (scene_.width <= 0.0 || scene_.height <= 0.0) {
    return QRectF(canvasRect.x() + rect.x, canvasRect.y() + rect.y, rect.width, rect.height);
  }

  const double scaleX = canvasRect.width() / scene_.width;
  const double scaleY = canvasRect.height() / scene_.height;
  return QRectF(canvasRect.x() + rect.x * scaleX, canvasRect.y() + rect.y * scaleY,
                rect.width * scaleX, rect.height * scaleY);
}

QPointF CanvasWidget::activeDragWidgetOffset() const {
  if (!dragActive_) {
    return QPointF(0.0, 0.0);
  }

  return dragCurrentWidgetPos_ - dragStartWidgetPos_;
}

QPointF CanvasWidget::scenePositionForWidgetPoint(const QPointF& widgetPoint) const {
  const QRectF canvasRect = canvasRectForWidget();
  if (scene_.width <= 0.0 || scene_.height <= 0.0) {
    return QPointF(0.0, 0.0);
  }

  const double scaleX = canvasRect.width() / scene_.width;
  const double scaleY = canvasRect.height() / scene_.height;
  return QPointF((widgetPoint.x() - canvasRect.x()) / scaleX, (widgetPoint.y() - canvasRect.y()) / scaleY);
}

void CanvasWidget::mousePressEvent(QMouseEvent* event) {
  if (event->button() == Qt::LeftButton) {
    const auto nodeId = pickNodeAt(event->position());
    if (!nodeId.isEmpty()) {
      if (nodeId == selectedNode_.id) {
        dragActive_ = true;
        dragNodeId_ = nodeId;
        dragStartWidgetPos_ = event->position();
        dragCurrentWidgetPos_ = event->position();
        dragStartSceneX_ = selectedNode_.positionX;
        dragStartSceneY_ = selectedNode_.positionY;
      } else {
        emit nodePicked(nodeId);
      }

      update();
      event->accept();
      return;
    }
  }

  QWidget::mousePressEvent(event);
}

void CanvasWidget::mouseMoveEvent(QMouseEvent* event) {
  if (dragActive_) {
    dragCurrentWidgetPos_ = event->position();
    const QPointF sceneStart = scenePositionForWidgetPoint(dragStartWidgetPos_);
    const QPointF sceneCurrent = scenePositionForWidgetPoint(dragCurrentWidgetPos_);
    emit nodeDragPreview(
        dragStartSceneX_ + (sceneCurrent.x() - sceneStart.x()),
        dragStartSceneY_ + (sceneCurrent.y() - sceneStart.y()));
    update();
    event->accept();
    return;
  }

  QWidget::mouseMoveEvent(event);
}

void CanvasWidget::mouseReleaseEvent(QMouseEvent* event) {
  if (dragActive_ && event->button() == Qt::LeftButton) {
    dragCurrentWidgetPos_ = event->position();
    const QPointF sceneStart = scenePositionForWidgetPoint(dragStartWidgetPos_);
    const QPointF sceneCurrent = scenePositionForWidgetPoint(dragCurrentWidgetPos_);
    emit nodeDragCommitted(
        dragNodeId_,
        dragStartSceneX_ + (sceneCurrent.x() - sceneStart.x()),
        dragStartSceneY_ + (sceneCurrent.y() - sceneStart.y()));
    dragActive_ = false;
    dragNodeId_.clear();
    update();
    event->accept();
    return;
  }

  QWidget::mouseReleaseEvent(event);
}

QString CanvasWidget::pickNodeAt(const QPointF& widgetPoint) const {
  const QRectF canvasRect = canvasRectForWidget();
  for (auto it = scene_.renderItems.crbegin(); it != scene_.renderItems.crend(); ++it) {
    if (!it->hasBounds) {
      continue;
    }

    if (mapSceneRect(it->bounds, canvasRect).contains(widgetPoint)) {
      return it->nodeId;
    }
  }

  return QString();
}

MainWindow::MainWindow(const QString& scenePath, QWidget* parent) : QMainWindow(parent) {
  buildUi();
  loadScene(scenePath);
}

void MainWindow::closeEvent(QCloseEvent* event) {
  if (!maybeResolveUnsavedChanges("close this window")) {
    event->ignore();
    return;
  }

  event->accept();
}

void MainWindow::buildUi() {
  setWindowTitle("tweaky");
  resize(1380, 900);
  buildMenus();

  canvas_ = new CanvasWidget(this);
  setCentralWidget(canvas_);
  connect(canvas_, &CanvasWidget::nodePicked, this, &MainWindow::handleCanvasNodePicked);
  connect(canvas_, &CanvasWidget::nodeDragPreview, this, &MainWindow::handleCanvasNodeDragPreview);
  connect(canvas_, &CanvasWidget::nodeDragCommitted, this,
          &MainWindow::handleCanvasNodeDragCommitted);

  hierarchyTree_ = new QTreeWidget(this);
  hierarchyTree_->setHeaderLabels({"Node", "Type"});
  hierarchyTree_->header()->setSectionResizeMode(QHeaderView::Stretch);
  connect(hierarchyTree_, &QTreeWidget::itemSelectionChanged, this,
          &MainWindow::handleTreeSelectionChanged);

  auto* hierarchyDock = new QDockWidget("Hierarchy", this);
  hierarchyDock->setWidget(hierarchyTree_);
  addDockWidget(Qt::LeftDockWidgetArea, hierarchyDock);

  auto* inspectorPanel = new QWidget(this);
  auto* inspectorLayout = new QVBoxLayout(inspectorPanel);
  auto* inspectorForm = new QFormLayout();
  nameEdit_ = new QLineEdit(inspectorPanel);
  nameEdit_->setPlaceholderText("Selected node name");
  inspectorForm->addRow("Name", nameEdit_);

  xSpin_ = new QDoubleSpinBox(inspectorPanel);
  xSpin_->setRange(-100000.0, 100000.0);
  xSpin_->setDecimals(2);
  inspectorForm->addRow("X", xSpin_);

  ySpin_ = new QDoubleSpinBox(inspectorPanel);
  ySpin_->setRange(-100000.0, 100000.0);
  ySpin_->setDecimals(2);
  inspectorForm->addRow("Y", ySpin_);

  paramsEdit_ = new QPlainTextEdit(inspectorPanel);
  paramsEdit_->setPlaceholderText("{\n  \"width\": 1360\n}");
  paramsEdit_->setFixedHeight(140);
  inspectorForm->addRow("Params JSON", paramsEdit_);

  styleEdit_ = new QPlainTextEdit(inspectorPanel);
  styleEdit_->setPlaceholderText("{\n  \"fill\": \"#dd6b42\"\n}");
  styleEdit_->setFixedHeight(140);
  inspectorForm->addRow("Style JSON", styleEdit_);
  inspectorLayout->addLayout(inspectorForm);

  autoApplyTimer_ = new QTimer(this);
  autoApplyTimer_->setSingleShot(true);
  autoApplyTimer_->setInterval(350);
  connect(autoApplyTimer_, &QTimer::timeout, this, &MainWindow::applyNodeEdits);

  applyEditsButton_ = new QPushButton("Apply Properties", inspectorPanel);
  inspectorLayout->addWidget(applyEditsButton_);
  connect(applyEditsButton_, &QPushButton::clicked, this, &MainWindow::applyNodeEdits);
  connect(nameEdit_, &QLineEdit::returnPressed, this, &MainWindow::applyNodeEdits);
  connect(nameEdit_, &QLineEdit::textEdited, this, &MainWindow::scheduleAutoApply);
  connect(xSpin_, &QDoubleSpinBox::valueChanged, this, &MainWindow::scheduleAutoApply);
  connect(ySpin_, &QDoubleSpinBox::valueChanged, this, &MainWindow::scheduleAutoApply);
  connect(paramsEdit_, &QPlainTextEdit::textChanged, this, &MainWindow::scheduleAutoApply);
  connect(styleEdit_, &QPlainTextEdit::textChanged, this, &MainWindow::scheduleAutoApply);

  inspectorText_ = new QTextEdit(this);
  inspectorText_->setReadOnly(true);
  inspectorText_->setPlaceholderText("Select a node to inspect its params and style.");
  inspectorLayout->addWidget(inspectorText_, 1);
  inspectorPanel->setLayout(inspectorLayout);

  auto* inspectorDock = new QDockWidget("Inspector", this);
  inspectorDock->setWidget(inspectorPanel);
  addDockWidget(Qt::RightDockWidgetArea, inspectorDock);

  statusBar()->showMessage("Ready");
}

void MainWindow::buildMenus() {
  auto* fileMenu = menuBar()->addMenu("&File");

  auto* openAction = fileMenu->addAction("&Open...");
  openAction->setShortcut(QKeySequence::Open);
  connect(openAction, &QAction::triggered, this, &MainWindow::openSceneDialog);

  auto* reloadAction = fileMenu->addAction("&Reload");
  reloadAction->setShortcut(QKeySequence(Qt::CTRL | Qt::Key_R));
  connect(reloadAction, &QAction::triggered, this, &MainWindow::reloadScene);

  fileMenu->addSeparator();

  auto* saveAction = fileMenu->addAction("&Save");
  saveAction->setShortcut(QKeySequence::Save);
  connect(saveAction, &QAction::triggered, this, &MainWindow::saveScene);

  auto* saveAsAction = fileMenu->addAction("Save &As...");
  saveAsAction->setShortcut(QKeySequence::SaveAs);
  connect(saveAsAction, &QAction::triggered, this, &MainWindow::saveSceneAs);

  fileMenu->addSeparator();

  auto* exportAction = fileMenu->addAction("Export &PNG...");
  exportAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_E));
  connect(exportAction, &QAction::triggered, this, &MainWindow::exportPngDialog);

  fileMenu->addSeparator();

  auto* quitAction = fileMenu->addAction("&Quit");
  quitAction->setShortcut(QKeySequence::Quit);
  connect(quitAction, &QAction::triggered, this, &QWidget::close);
}

bool MainWindow::loadScene(const QString& scenePath) {
  if (!ensureWorkingCopyFromSource(scenePath)) {
    statusBar()->showMessage(QString("Failed to prepare working copy for %1").arg(scenePath));
    inspectorText_->setPlainText(QString("Failed to prepare working copy:\n%1").arg(scenePath));
    return false;
  }

  if (loadSceneFromEditorCli(scene_.workingPath, scenePath)) {
    refreshUiAfterSceneLoad(QString("Loaded %1 via editor view-model").arg(scenePath));
    markDirty(false);
    return true;
  }

  if (loadSceneFromRawJson(scene_.workingPath, scenePath)) {
    refreshUiAfterSceneLoad(QString("Loaded %1 via raw JSON fallback").arg(scenePath));
    markDirty(false);
    return true;
  }

  statusBar()->showMessage(QString("Failed to load %1").arg(scenePath));
  inspectorText_->setPlainText(QString("Failed to load scene file:\n%1").arg(scenePath));
  return false;
}

void MainWindow::openSceneDialog() {
  if (!maybeResolveUnsavedChanges("open another scene")) {
    return;
  }

  const QString startPath =
      scene_.sourcePath.isEmpty() ? QDir::currentPath() : QFileInfo(scene_.sourcePath).absolutePath();
  const auto filePath = QFileDialog::getOpenFileName(
      this, "Open Scene", startPath, "Tweaky Scene (*.vsd.json);;JSON Files (*.json)");

  if (filePath.isEmpty()) {
    return;
  }

  if (!loadScene(filePath)) {
    QMessageBox::warning(this, "Unable to Open Scene",
                         QString("tweaky could not open:\n%1").arg(filePath));
  }
}

void MainWindow::reloadScene() {
  if (scene_.sourcePath.isEmpty()) {
    QMessageBox::information(this, "Nothing to Reload",
                             "No scene file is currently loaded.");
    return;
  }

  if (!maybeResolveUnsavedChanges("reload from disk")) {
    return;
  }

  if (!loadScene(scene_.sourcePath)) {
    QMessageBox::warning(this, "Reload Failed",
                         QString("tweaky could not reload:\n%1").arg(scene_.sourcePath));
  }
}

void MainWindow::saveScene() {
  if (scene_.sourcePath.isEmpty()) {
    saveSceneAs();
    return;
  }

  if (!saveWorkingCopyToPath(scene_.sourcePath)) {
    QMessageBox::warning(this, "Save Failed",
                         QString("tweaky could not save:\n%1").arg(scene_.sourcePath));
    return;
  }

  markDirty(false);
  statusBar()->showMessage(QString("Saved %1").arg(scene_.sourcePath), 3000);
}

void MainWindow::saveSceneAs() {
  if (scene_.workingPath.isEmpty()) {
    QMessageBox::information(this, "Nothing to Save",
                             "Load a scene before saving.");
    return;
  }

  const QFileInfo sceneFileInfo(scene_.sourcePath.isEmpty() ? scene_.workingPath : scene_.sourcePath);
  const auto outputPath = QFileDialog::getSaveFileName(
      this, "Save Scene As", sceneFileInfo.absoluteFilePath(),
      "Tweaky Scene (*.vsd.json);;JSON Files (*.json)");

  if (outputPath.isEmpty()) {
    return;
  }

  if (!saveWorkingCopyToPath(outputPath)) {
    QMessageBox::warning(this, "Save As Failed",
                         QString("tweaky could not save:\n%1").arg(outputPath));
    return;
  }

  scene_.sourcePath = outputPath;
  markDirty(false);
  statusBar()->showMessage(QString("Saved %1").arg(outputPath), 3000);
}

void MainWindow::exportPngDialog() {
  if (scene_.sourcePath.isEmpty()) {
    QMessageBox::information(this, "Nothing to Export",
                             "Load a scene before exporting a PNG.");
    return;
  }

  const QFileInfo sceneFileInfo(scene_.sourcePath);
  const QString defaultPath =
      sceneFileInfo.absoluteDir().filePath(sceneFileInfo.completeBaseName() + ".png");
  const auto outputPath =
      QFileDialog::getSaveFileName(this, "Export PNG", defaultPath, "PNG Image (*.png)");

  if (outputPath.isEmpty()) {
    return;
  }

  if (!exportSceneToPng(outputPath)) {
    QMessageBox::warning(this, "Export Failed",
                         QString("tweaky could not export a PNG to:\n%1").arg(outputPath));
  }
}

void MainWindow::scheduleAutoApply() {
  if (suppressInspectorSignals_ || scene_.selectedNodeId.isEmpty()) {
    return;
  }

  autoApplyTimer_->start();
}

void MainWindow::applyNodeEdits() {
  if (scene_.selectedNodeId.isEmpty()) {
    return;
  }

  const QString newName = nameEdit_->text().trimmed();
  if (newName.isEmpty()) {
    return;
  }

  QString validationError;
  if (!inspectorJsonIsValid(&validationError)) {
    statusBar()->showMessage(validationError, 2500);
    return;
  }

  const QString paramsJson = paramsEdit_->toPlainText().trimmed();
  const QString styleJson = styleEdit_->toPlainText().trimmed();

  if (!applyNodePropertyEdits(scene_.selectedNodeId, newName, xSpin_->value(), ySpin_->value(),
                              paramsJson, styleJson)) {
    statusBar()->showMessage(QString("Failed to update node %1").arg(scene_.selectedNodeId),
                             3000);
    return;
  }

  refreshUiAfterSceneLoad(QString("Updated node %1").arg(newName));
}

void MainWindow::updateWindowTitle() {
  const auto sourceName =
      scene_.sourcePath.isEmpty() ? QString("untitled") : QFileInfo(scene_.sourcePath).fileName();
  const QString dirtyMarker = scene_.dirty ? QString(" *") : QString();
  setWindowTitle(QString("tweaky - %1 (%2)%3").arg(scene_.name, sourceName, dirtyMarker));
}

bool MainWindow::exportSceneToPng(const QString& outputPath) {
  if (scene_.workingPath.isEmpty()) {
    return false;
  }

  QProcess process(this);
  process.setProgram(editorCliPath());
  process.setArguments({scene_.workingPath, "--export", outputPath});
  process.start();

  if (!process.waitForStarted(2000)) {
    return false;
  }

  if (!process.waitForFinished(15000) || process.exitStatus() != QProcess::NormalExit ||
      process.exitCode() != 0) {
    const QString stderrText = QString::fromUtf8(process.readAllStandardError()).trimmed();
    if (!stderrText.isEmpty()) {
      inspectorText_->setPlainText(stderrText);
    }
    return false;
  }

  statusBar()->showMessage(QString("Exported PNG to %1").arg(outputPath), 4000);
  return true;
}

bool MainWindow::applyNodePropertyEdits(const QString& nodeId, const QString& newName, double x,
                                        double y, const QString& paramsJson,
                                        const QString& styleJson) {
  if (scene_.workingPath.isEmpty()) {
    return false;
  }

  const bool loaded = loadSceneFromEditorCli(
      scene_.workingPath, scene_.sourcePath,
      {"--rename-node", nodeId, newName, "--set-position", nodeId,
       QString::number(x, 'f', 2), QString::number(y, 'f', 2), "--set-params-json", nodeId,
       paramsJson, "--set-style-json", nodeId, styleJson});
  if (loaded) {
    markDirty(true);
  }
  return loaded;
}

bool MainWindow::loadSceneFromEditorCli(const QString& scenePath, const QString& sourcePath,
                                        const QStringList& extraArgs) {
  QProcess process(this);
  process.setProgram(editorCliPath());
  QStringList arguments = {scenePath};
  arguments.append(extraArgs);
  arguments.append("--dump-view-model");
  process.setArguments(arguments);
  process.start();

  if (!process.waitForStarted(2000)) {
    return false;
  }

  if (!process.waitForFinished(5000) || process.exitStatus() != QProcess::NormalExit ||
      process.exitCode() != 0) {
    return false;
  }

  const auto document = QJsonDocument::fromJson(process.readAllStandardOutput());
  if (!document.isObject()) {
    return false;
  }

  const auto root = document.object();
  scene_.workingPath = root.value("document_path").toString(scenePath);
  scene_.sourcePath = sourcePath.isEmpty() ? scene_.workingPath : sourcePath;
  scene_.name = root.value("document_name").toString("Untitled");
  scene_.width = root.value("canvas_width").toDouble();
  scene_.height = root.value("canvas_height").toDouble();
  scene_.background = QColor(root.value("background").toString("#f5f1e8"));
  scene_.selectedNodeId = root.value("selected_node_id").toString();
  scene_.nodes.clear();
  scene_.renderItems.clear();
  nodeIndex_.clear();

  const auto nodes = root.value("nodes").toArray();
  for (const auto& nodeValue : nodes) {
    const auto object = nodeValue.toObject();
    SceneNodeData node;
    node.depth = object.value("depth").toInt();
    node.id = object.value("id").toString();
    node.type = object.value("node_type").toString();
    node.name = object.value("name").toString();
    node.positionX = object.value("position_x").toDouble();
    node.positionY = object.value("position_y").toDouble();
    node.params = object.value("params").toObject();
    node.style = object.value("style").toObject();
    const auto bounds = object.value("bounds").toObject();
    node.hasBounds = !bounds.isEmpty();
    node.bounds.x = bounds.value("x").toDouble();
    node.bounds.y = bounds.value("y").toDouble();
    node.bounds.width = bounds.value("width").toDouble();
    node.bounds.height = bounds.value("height").toDouble();
    scene_.nodes.push_back(node);
    nodeIndex_.insert(node.id, node);
  }

  const auto renderItems = root.value("render_items").toArray();
  for (const auto& itemValue : renderItems) {
    const auto object = itemValue.toObject();
    SceneCanvasItemData item;
    item.nodeId = object.value("node_id").toString();
    item.kind = object.value("kind").toString();
    item.opacity = object.value("opacity").toDouble(1.0);
    item.blendMode = object.value("blend_mode").toString();
    const auto bounds = object.value("bounds").toObject();
    item.hasBounds = !bounds.isEmpty();
    item.bounds.x = bounds.value("x").toDouble();
    item.bounds.y = bounds.value("y").toDouble();
    item.bounds.width = bounds.value("width").toDouble();
    item.bounds.height = bounds.value("height").toDouble();
    item.fill = QColor(object.value("fill").toString());
    item.cornerRadius = object.value("corner_radius").toDouble();

    const auto origin = object.value("origin").toObject();
    item.hasOrigin = !origin.isEmpty();
    item.origin.x = origin.value("x").toDouble();
    item.origin.y = origin.value("y").toDouble();

    const auto points = object.value("points").toArray();
    for (const auto& pointValue : points) {
      const auto pointObject = pointValue.toObject();
      item.points.push_back(
          ScenePointData{pointObject.value("x").toDouble(), pointObject.value("y").toDouble()});
    }

    item.closed = object.value("closed").toBool(true);
    item.text = object.value("text").toString();
    item.fontSize = object.value("font_size").toDouble(12.0);
    item.fontFamily = object.value("font_family").toString();
    item.imageRef = object.value("image_ref").toString();
    item.blurRadius = object.value("blur_radius").toDouble(0.0);

    const auto shadow = object.value("shadow").toObject();
    item.hasShadow = !shadow.isEmpty();
    item.shadow.color = QColor(shadow.value("color").toString());
    item.shadow.offsetX = shadow.value("offset_x").toDouble();
    item.shadow.offsetY = shadow.value("offset_y").toDouble();
    item.shadow.blurRadius = shadow.value("blur_radius").toDouble();

    scene_.renderItems.push_back(item);
  }

  return !scene_.nodes.isEmpty();
}

bool MainWindow::loadSceneFromRawJson(const QString& scenePath, const QString& sourcePath) {
  QFile file(scenePath);
  if (!file.open(QIODevice::ReadOnly)) {
    return false;
  }

  const auto data = file.readAll();
  const auto document = QJsonDocument::fromJson(data);
  const auto rootObject = document.object();
  const auto sceneObject = rootObject.value("document").toObject();

  scene_.workingPath = scenePath;
  scene_.sourcePath = sourcePath.isEmpty() ? scenePath : sourcePath;
  scene_.name = sceneObject.value("name").toString("Untitled");
  scene_.width = sceneObject.value("width").toDouble();
  scene_.height = sceneObject.value("height").toDouble();
  const auto background = sceneObject.value("background").toObject();
  scene_.background = QColor(background.value("color").toString("#f5f1e8"));
  scene_.selectedNodeId = "root";
  scene_.nodes.clear();
  scene_.renderItems.clear();
  nodeIndex_.clear();

  std::function<void(const QJsonObject&, int)> collect = [&](const QJsonObject& node, int depth) {
    SceneNodeData data;
    data.depth = depth;
    data.id = node.value("id").toString();
    data.type = node.value("type").toString();
    data.name = node.value("name").toString();
    const auto transform = node.value("transform").toObject();
    data.positionX = transform.value("x").toDouble();
    data.positionY = transform.value("y").toDouble();
    data.params = node.value("params").toObject();
    data.style = node.value("style").toObject();
    scene_.nodes.push_back(data);
    nodeIndex_.insert(data.id, data);

    const auto children = node.value("children").toArray();
    for (const auto& childValue : children) {
      collect(childValue.toObject(), depth + 1);
    }
  };

  collect(sceneObject.value("root").toObject(), 0);
  return !scene_.nodes.isEmpty();
}

void MainWindow::populateTree() {
  hierarchyTree_->clear();
  QList<QTreeWidgetItem*> depthParents;
  for (const auto& node : scene_.nodes) {
    auto* item = new QTreeWidgetItem();
    populateTreeNode(item, node);

    const int nodeDepth = node.depth;

    while (depthParents.size() > nodeDepth) {
      depthParents.removeLast();
    }

    if (depthParents.isEmpty()) {
      hierarchyTree_->addTopLevelItem(item);
    } else {
      depthParents.last()->addChild(item);
    }

    depthParents.push_back(item);
  }

  for (int i = 0; i < hierarchyTree_->topLevelItemCount(); ++i) {
    hierarchyTree_->topLevelItem(i)->setExpanded(true);
  }

  if (auto* selectedItem = findTreeItemByNodeId(scene_.selectedNodeId)) {
    hierarchyTree_->setCurrentItem(selectedItem);
  } else if (hierarchyTree_->topLevelItemCount() > 0) {
    hierarchyTree_->setCurrentItem(hierarchyTree_->topLevelItem(0));
  }
}

void MainWindow::populateTreeNode(QTreeWidgetItem* item, const SceneNodeData& node) {
  item->setText(0, node.name);
  item->setText(1, node.type);
  item->setData(0, Qt::UserRole, node.id);
  item->setToolTip(0, node.id);
}

void MainWindow::handleTreeSelectionChanged() {
  const auto items = hierarchyTree_->selectedItems();
  if (items.isEmpty()) {
    return;
  }

  const auto id = items.first()->data(0, Qt::UserRole).toString();
  if (!nodeIndex_.contains(id)) {
    return;
  }

  const auto node = nodeIndex_.value(id);
  scene_.selectedNodeId = id;
  updateInspector(node);
  canvas_->setSelectedNode(node);
  statusBar()->showMessage(QString("Selected %1 [%2]").arg(node.name, node.type));
}

void MainWindow::handleCanvasNodePicked(const QString& nodeId) {
  if (auto* selectedItem = findTreeItemByNodeId(nodeId)) {
    hierarchyTree_->setCurrentItem(selectedItem);
  }
}

void MainWindow::handleCanvasNodeDragPreview(double x, double y) {
  suppressInspectorSignals_ = true;
  xSpin_->setValue(x);
  ySpin_->setValue(y);
  suppressInspectorSignals_ = false;
}

void MainWindow::handleCanvasNodeDragCommitted(const QString& nodeId, double x, double y) {
  if (nodeId != scene_.selectedNodeId) {
    return;
  }

  suppressInspectorSignals_ = true;
  xSpin_->setValue(x);
  ySpin_->setValue(y);
  suppressInspectorSignals_ = false;
  applyNodeEdits();
}

void MainWindow::updateInspector(const SceneNodeData& node) {
  populateInspectorFields(node);
  QStringList sections;
  sections << QString("id: %1").arg(node.id);
  sections << QString("type: %1").arg(node.type);
  sections << QString("name: %1").arg(node.name);
  sections << "";
  sections << "params:";
  sections << objectToPrettyJson(node.params);
  sections << "";
  sections << "style:";
  sections << objectToPrettyJson(node.style);
  inspectorText_->setPlainText(sections.join("\n"));
}

void MainWindow::populateInspectorFields(const SceneNodeData& node) {
  suppressInspectorSignals_ = true;
  nameEdit_->setText(node.name);
  xSpin_->setValue(node.positionX);
  ySpin_->setValue(node.positionY);
  paramsEdit_->setPlainText(objectToPrettyJson(node.params));
  styleEdit_->setPlainText(objectToPrettyJson(node.style));
  suppressInspectorSignals_ = false;
}

QString MainWindow::objectToPrettyJson(const QJsonObject& object) const {
  return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Indented)).trimmed();
}

void MainWindow::refreshUiAfterSceneLoad(const QString& statusMessage) {
  updateWindowTitle();
  canvas_->setScene(scene_);
  populateTree();
  statusBar()->showMessage(statusMessage, 2500);
}

void MainWindow::markDirty(bool dirty) {
  scene_.dirty = dirty;
  updateWindowTitle();
}

bool MainWindow::ensureWorkingCopyFromSource(const QString& sourcePath) {
  if (!workingCopyDirectory_.isValid()) {
    return false;
  }

  const QString uniqueName = QString("%1-%2")
                                 .arg(QUuid::createUuid().toString(QUuid::WithoutBraces),
                                      QFileInfo(sourcePath).fileName());
  const QString workingPath = workingCopyDirectory_.filePath(uniqueName);
  QFile::remove(workingPath);
  if (!QFile::copy(sourcePath, workingPath)) {
    return false;
  }

  scene_.sourcePath = sourcePath;
  scene_.workingPath = workingPath;
  return true;
}

bool MainWindow::saveWorkingCopyToPath(const QString& outputPath) {
  if (scene_.workingPath.isEmpty()) {
    return false;
  }

  QFile inputFile(scene_.workingPath);
  if (!inputFile.open(QIODevice::ReadOnly)) {
    return false;
  }

  QSaveFile outputFile(outputPath);
  if (!outputFile.open(QIODevice::WriteOnly)) {
    return false;
  }

  if (outputFile.write(inputFile.readAll()) < 0) {
    outputFile.cancelWriting();
    return false;
  }

  return outputFile.commit();
}

bool MainWindow::maybeResolveUnsavedChanges(const QString& actionLabel) {
  if (!scene_.dirty) {
    return true;
  }

  QMessageBox messageBox(this);
  messageBox.setIcon(QMessageBox::Warning);
  messageBox.setWindowTitle("Unsaved Changes");
  messageBox.setText("You have unsaved changes.");
  messageBox.setInformativeText(QString("Do you want to save before you %1?").arg(actionLabel));
  auto* saveButton = messageBox.addButton(QMessageBox::Save);
  auto* discardButton = messageBox.addButton(QMessageBox::Discard);
  auto* cancelButton = messageBox.addButton(QMessageBox::Cancel);
  messageBox.setDefaultButton(qobject_cast<QPushButton*>(saveButton));
  messageBox.exec();

  if (messageBox.clickedButton() == cancelButton) {
    return false;
  }

  if (messageBox.clickedButton() == discardButton) {
    return true;
  }

  if (messageBox.clickedButton() == saveButton) {
    if (scene_.sourcePath.isEmpty()) {
      saveSceneAs();
    } else {
      saveScene();
    }
    return !scene_.dirty;
  }

  return true;
}

bool MainWindow::inspectorJsonIsValid(QString* errorMessage) const {
  const auto paramsDocument = QJsonDocument::fromJson(paramsEdit_->toPlainText().trimmed().toUtf8());
  if (!paramsDocument.isObject()) {
    if (errorMessage != nullptr) {
      *errorMessage = "Params must be a valid JSON object.";
    }
    return false;
  }

  const auto styleDocument = QJsonDocument::fromJson(styleEdit_->toPlainText().trimmed().toUtf8());
  if (!styleDocument.isObject()) {
    if (errorMessage != nullptr) {
      *errorMessage = "Style must be a valid JSON object.";
    }
    return false;
  }

  return true;
}

QTreeWidgetItem* MainWindow::findTreeItemByNodeId(const QString& nodeId) const {
  QTreeWidgetItemIterator iterator(hierarchyTree_);
  while (*iterator != nullptr) {
    if ((*iterator)->data(0, Qt::UserRole).toString() == nodeId) {
      return *iterator;
    }
    ++iterator;
  }

  return nullptr;
}

QString MainWindow::editorCliPath() const {
  const auto fromEnv = qEnvironmentVariable("TWEAKY_EDITOR_CLI");
  if (!fromEnv.isEmpty()) {
    return fromEnv;
  }

  const auto appDir = QApplication::applicationDirPath();
  const QStringList candidates = {
      QDir::current().filePath("target/debug/editor"),
      QDir(appDir).filePath("editor"),
      QDir(appDir).filePath("../../../../target/debug/editor"),
      QDir(appDir).filePath("../../../../target/release/editor"),
  };

  for (const auto& candidate : candidates) {
    if (QFileInfo::exists(candidate)) {
      return QDir::cleanPath(candidate);
    }
  }

  return candidates.first();
}
