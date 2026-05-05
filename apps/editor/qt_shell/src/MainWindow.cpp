#include "MainWindow.h"

#include <QDockWidget>
#include <QFile>
#include <QHeaderView>
#include <QJsonDocument>
#include <QJsonValue>
#include <QLabel>
#include <QPainter>
#include <QStatusBar>
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

  QRectF canvasRect(40.0, 40.0, width() - 80.0, height() - 80.0);
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

  const QString summary = QString("Scene: %1 x %2\nSelected: %3 [%4]")
                              .arg(scene_.width, 0, 'f', 0)
                              .arg(scene_.height, 0, 'f', 0)
                              .arg(selectedNode_.name.isEmpty() ? QString("None") : selectedNode_.name)
                              .arg(selectedNode_.type.isEmpty() ? QString("-") : selectedNode_.type);

  painter.drawText(QRectF(56.0, 108.0, canvasRect.width() - 64.0, 72.0), summary);

  QRectF focusRect(96.0, 210.0, canvasRect.width() * 0.48, canvasRect.height() * 0.34);
  painter.setPen(QPen(QColor("#dd6b42"), 3.0, Qt::DashLine));
  painter.setBrush(Qt::NoBrush);
  painter.drawRoundedRect(focusRect, 16.0, 16.0);

  painter.setPen(QColor("#6b5a4f"));
  painter.drawText(QRectF(focusRect.x(), focusRect.bottom() + 12.0, 320.0, 24.0),
                   QString("Interactive canvas host will live here"));
}

MainWindow::MainWindow(const QString& scenePath, QWidget* parent) : QMainWindow(parent) {
  buildUi();
  loadScene(scenePath);
}

void MainWindow::buildUi() {
  setWindowTitle("tweaky");
  resize(1380, 900);

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

  inspectorText_ = new QTextEdit(this);
  inspectorText_->setReadOnly(true);
  inspectorText_->setPlaceholderText("Select a node to inspect its params and style.");

  auto* inspectorDock = new QDockWidget("Inspector", this);
  inspectorDock->setWidget(inspectorText_);
  addDockWidget(Qt::RightDockWidgetArea, inspectorDock);

  statusBar()->showMessage("Ready");
}

void MainWindow::loadScene(const QString& scenePath) {
  QFile file(scenePath);
  if (!file.open(QIODevice::ReadOnly)) {
    statusBar()->showMessage(QString("Failed to open %1").arg(scenePath));
    inspectorText_->setPlainText(QString("Failed to open scene file:\n%1").arg(scenePath));
    return;
  }

  const auto data = file.readAll();
  const auto document = QJsonDocument::fromJson(data);
  const auto rootObject = document.object();
  const auto sceneObject = rootObject.value("document").toObject();

  scene_.sourcePath = scenePath;
  scene_.name = sceneObject.value("name").toString("Untitled");
  scene_.width = sceneObject.value("width").toDouble();
  scene_.height = sceneObject.value("height").toDouble();
  scene_.root = sceneObject.value("root").toObject();
  const auto background = sceneObject.value("background").toObject();
  scene_.background = QColor(background.value("color").toString("#f5f1e8"));

  setWindowTitle(QString("tweaky - %1").arg(scene_.name));
  canvas_->setScene(scene_);
  populateTree();
  statusBar()->showMessage(QString("Loaded %1").arg(scenePath));
}

void MainWindow::populateTree() {
  hierarchyTree_->clear();

  auto* rootItem = new QTreeWidgetItem(hierarchyTree_);
  populateTreeNode(rootItem, scene_.root, 0);
  hierarchyTree_->addTopLevelItem(rootItem);
  rootItem->setExpanded(true);
  hierarchyTree_->setCurrentItem(rootItem);
}

void MainWindow::populateTreeNode(QTreeWidgetItem* item, const QJsonObject& node, int depth) {
  Q_UNUSED(depth);

  item->setText(0, node.value("name").toString("Unnamed"));
  item->setText(1, node.value("type").toString("Unknown"));
  item->setData(0, Qt::UserRole, node);

  const auto children = node.value("children").toArray();
  for (const auto& childValue : children) {
    auto* childItem = new QTreeWidgetItem(item);
    populateTreeNode(childItem, childValue.toObject(), depth + 1);
  }
}

void MainWindow::handleTreeSelectionChanged() {
  const auto items = hierarchyTree_->selectedItems();
  if (items.isEmpty()) {
    return;
  }

  const auto object = items.first()->data(0, Qt::UserRole).toJsonObject();
  const auto node = nodeDataFromObject(object);
  updateInspector(node);
  canvas_->setSelectedNode(node);
}

void MainWindow::updateInspector(const SceneNodeData& node) {
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

SceneNodeData MainWindow::nodeDataFromObject(const QJsonObject& node) const {
  SceneNodeData data;
  data.id = node.value("id").toString();
  data.type = node.value("type").toString();
  data.name = node.value("name").toString();
  data.params = node.value("params").toObject();
  data.style = node.value("style").toObject();
  return data;
}

QString MainWindow::objectToPrettyJson(const QJsonObject& object) const {
  return QString::fromUtf8(QJsonDocument(object).toJson(QJsonDocument::Indented)).trimmed();
}
