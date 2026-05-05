#pragma once

#include <QJsonArray>
#include <QJsonObject>
#include <QMainWindow>
#include <QTreeWidget>
#include <QTextEdit>
#include <QWidget>

struct SceneNodeData {
    QString id;
    QString type;
    QString name;
    QJsonObject params;
    QJsonObject style;
};

struct SceneDocumentData {
    QString name;
    double width = 0.0;
    double height = 0.0;
    QColor background = QColor("#f5f1e8");
    QString sourcePath;
    QJsonObject root;
};

class CanvasWidget : public QWidget {
  Q_OBJECT

public:
  explicit CanvasWidget(QWidget* parent = nullptr);

  void setScene(const SceneDocumentData& scene);
  void setSelectedNode(const SceneNodeData& node);

protected:
  void paintEvent(QPaintEvent* event) override;

private:
  SceneDocumentData scene_;
  SceneNodeData selectedNode_;
};

class MainWindow : public QMainWindow {
  Q_OBJECT

public:
  explicit MainWindow(const QString& scenePath, QWidget* parent = nullptr);

private slots:
  void handleTreeSelectionChanged();

private:
  void buildUi();
  void loadScene(const QString& scenePath);
  void populateTree();
  void populateTreeNode(QTreeWidgetItem* parentItem, const QJsonObject& node, int depth);
  void updateInspector(const SceneNodeData& node);
  SceneNodeData nodeDataFromObject(const QJsonObject& node) const;
  QString objectToPrettyJson(const QJsonObject& object) const;

  SceneDocumentData scene_;
  QTreeWidget* hierarchyTree_ = nullptr;
  QTextEdit* inspectorText_ = nullptr;
  CanvasWidget* canvas_ = nullptr;
};
