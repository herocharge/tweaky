#include "MainWindow.h"

#include <QAction>
#include <QApplication>
#include <QDockWidget>
#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QHeaderView>
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
#include <QStatusBar>
#include <QTreeWidgetItemIterator>
#include <QTreeWidgetItem>
#include <QVBoxLayout>

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

  for (const auto& item : scene_.renderItems) {
    const QColor fill = item.fill.isValid() ? item.fill : QColor("#c8bfb1");
    QColor stroke = fill.darker(135);
    stroke.setAlphaF(0.9);
    painter.setOpacity(item.opacity);

    if (item.kind == "Rectangle" && item.hasBounds) {
      painter.setPen(QPen(stroke, 1.5));
      painter.setBrush(fill);
      painter.drawRoundedRect(mapSceneRect(item.bounds, canvasRect), item.cornerRadius,
                              item.cornerRadius);
    } else if (item.kind == "Ellipse" && item.hasBounds) {
      painter.setPen(QPen(stroke, 1.5));
      painter.setBrush(fill);
      painter.drawEllipse(mapSceneRect(item.bounds, canvasRect));
    } else if (item.kind == "Path" && !item.points.isEmpty()) {
      QPainterPath path;
      path.moveTo(mapScenePoint(item.points.first(), canvasRect));
      for (qsizetype index = 1; index < item.points.size(); ++index) {
        path.lineTo(mapScenePoint(item.points.at(index), canvasRect));
      }
      if (item.closed) {
        path.closeSubpath();
      }
      painter.setPen(QPen(stroke, 2.0));
      painter.setBrush(item.closed ? fill : Qt::NoBrush);
      painter.drawPath(path);
    } else if (item.kind == "Text" && item.hasOrigin) {
      QFont textFont = painter.font();
      textFont.setPointSizeF(item.fontSize <= 0.0 ? 18.0 : item.fontSize);
      if (!item.fontFamily.isEmpty()) {
        textFont.setFamily(item.fontFamily);
      }
      painter.setFont(textFont);
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

    if (item.nodeId == scene_.selectedNodeId && item.hasBounds) {
      painter.setOpacity(1.0);
      painter.setPen(QPen(QColor("#dd6b42"), 2.5, Qt::DashLine));
      painter.setBrush(Qt::NoBrush);
      painter.drawRect(mapSceneRect(item.bounds, canvasRect).adjusted(-3.0, -3.0, 3.0, 3.0));
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

MainWindow::MainWindow(const QString& scenePath, QWidget* parent) : QMainWindow(parent) {
  buildUi();
  loadScene(scenePath);
}

void MainWindow::buildUi() {
  setWindowTitle("tweaky");
  resize(1380, 900);
  buildMenus();

  canvas_ = new CanvasWidget(this);
  setCentralWidget(canvas_);

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
  auto* renameForm = new QFormLayout();
  nameEdit_ = new QLineEdit(inspectorPanel);
  nameEdit_->setPlaceholderText("Selected node name");
  renameForm->addRow("Name", nameEdit_);
  textEdit_ = new QPlainTextEdit(inspectorPanel);
  textEdit_->setPlaceholderText("Text content for text nodes");
  textEdit_->setFixedHeight(88);
  renameForm->addRow("Text", textEdit_);
  fillEdit_ = new QLineEdit(inspectorPanel);
  fillEdit_->setPlaceholderText("#RRGGBB or #RRGGBBAA");
  renameForm->addRow("Fill", fillEdit_);
  inspectorLayout->addLayout(renameForm);

  applyEditsButton_ = new QPushButton("Apply Properties", inspectorPanel);
  inspectorLayout->addWidget(applyEditsButton_);
  connect(applyEditsButton_, &QPushButton::clicked, this, &MainWindow::applyNodeEdits);
  connect(nameEdit_, &QLineEdit::returnPressed, this, &MainWindow::applyNodeEdits);
  connect(fillEdit_, &QLineEdit::returnPressed, this, &MainWindow::applyNodeEdits);

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

  auto* exportAction = fileMenu->addAction("Export &PNG...");
  exportAction->setShortcut(QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_E));
  connect(exportAction, &QAction::triggered, this, &MainWindow::exportPngDialog);

  fileMenu->addSeparator();

  auto* quitAction = fileMenu->addAction("&Quit");
  quitAction->setShortcut(QKeySequence::Quit);
  connect(quitAction, &QAction::triggered, this, &QWidget::close);
}

bool MainWindow::loadScene(const QString& scenePath) {
  if (loadSceneFromEditorCli(scenePath)) {
    updateWindowTitle();
    canvas_->setScene(scene_);
    populateTree();
    statusBar()->showMessage(QString("Loaded %1 via editor view-model").arg(scenePath));
    return true;
  }

  if (loadSceneFromRawJson(scenePath)) {
    updateWindowTitle();
    canvas_->setScene(scene_);
    populateTree();
    statusBar()->showMessage(QString("Loaded %1 via raw JSON fallback").arg(scenePath));
    return true;
  }

  statusBar()->showMessage(QString("Failed to load %1").arg(scenePath));
  inspectorText_->setPlainText(QString("Failed to load scene file:\n%1").arg(scenePath));
  return false;
}

void MainWindow::openSceneDialog() {
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

  if (!loadScene(scene_.sourcePath)) {
    QMessageBox::warning(this, "Reload Failed",
                         QString("tweaky could not reload:\n%1").arg(scene_.sourcePath));
  }
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

void MainWindow::applyNodeEdits() {
  if (scene_.selectedNodeId.isEmpty()) {
    QMessageBox::information(this, "Nothing Selected",
                             "Select a node before applying edits.");
    return;
  }

  const QString newName = nameEdit_->text().trimmed();
  if (newName.isEmpty()) {
    QMessageBox::information(this, "Empty Name", "Node names cannot be empty.");
    return;
  }

  const QString textValue = textEdit_->toPlainText();
  const QString fillValue = fillEdit_->text().trimmed();
  if (!fillValue.isEmpty() && !fillValue.startsWith('#')) {
    QMessageBox::information(this, "Invalid Fill",
                             "Fill should be a hex color like #dd6b42.");
    return;
  }

  if (!applyNodePropertyEdits(scene_.selectedNodeId, newName, textValue, fillValue)) {
    QMessageBox::warning(this, "Apply Failed",
                         QString("tweaky could not update node %1.")
                             .arg(scene_.selectedNodeId));
    return;
  }

  if (!loadScene(scene_.sourcePath)) {
    QMessageBox::warning(this, "Reload Failed",
                         QString("tweaky renamed the node but failed to reload:\n%1")
                             .arg(scene_.sourcePath));
    return;
  }

  if (auto* selectedItem = findTreeItemByNodeId(scene_.selectedNodeId)) {
    hierarchyTree_->setCurrentItem(selectedItem);
  }
  statusBar()->showMessage(QString("Updated node %1").arg(newName), 4000);
}

void MainWindow::updateWindowTitle() {
  const auto sourceName =
      scene_.sourcePath.isEmpty() ? QString("untitled") : QFileInfo(scene_.sourcePath).fileName();
  setWindowTitle(QString("tweaky - %1 (%2)").arg(scene_.name, sourceName));
}

bool MainWindow::exportSceneToPng(const QString& outputPath) {
  if (scene_.sourcePath.isEmpty()) {
    return false;
  }

  QProcess process(this);
  process.setProgram(editorCliPath());
  process.setArguments({scene_.sourcePath, "--export", outputPath});
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

bool MainWindow::applyNodePropertyEdits(const QString& nodeId, const QString& newName,
                                        const QString& textValue, const QString& fillValue) {
  if (scene_.sourcePath.isEmpty()) {
    return false;
  }

  QProcess process(this);
  process.setProgram(editorCliPath());
  QStringList arguments = {scene_.sourcePath, "--rename-node", nodeId, newName};

  const auto selectedNode = nodeIndex_.value(nodeId);
  if (selectedNode.params.contains("text")) {
    arguments << "--set-text" << nodeId << textValue;
  }
  if (!fillValue.isEmpty() || selectedNode.style.contains("fill")) {
    arguments << "--set-fill" << nodeId << fillValue;
  }

  process.setArguments(arguments);
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

  return true;
}

bool MainWindow::loadSceneFromEditorCli(const QString& scenePath) {
  QProcess process(this);
  process.setProgram(editorCliPath());
  process.setArguments({scenePath, "--dump-view-model"});
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
  scene_.sourcePath = root.value("document_path").toString(scenePath);
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

bool MainWindow::loadSceneFromRawJson(const QString& scenePath) {
  QFile file(scenePath);
  if (!file.open(QIODevice::ReadOnly)) {
    return false;
  }

  const auto data = file.readAll();
  const auto document = QJsonDocument::fromJson(data);
  const auto rootObject = document.object();
  const auto sceneObject = rootObject.value("document").toObject();

  scene_.sourcePath = scenePath;
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
  nameEdit_->setText(node.name);
  textEdit_->setPlainText(node.params.value("text").toString());
  fillEdit_->setText(node.style.value("fill").toString());
}

QString MainWindow::objectToPrettyJson(const QJsonObject& object) const {
  return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Indented)).trimmed();
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
