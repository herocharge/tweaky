#pragma once

#include <QColor>
#include <QJsonArray>
#include <QJsonObject>
#include <QMainWindow>
#include <QMap>
#include <QRectF>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTreeWidget>
#include <QTextEdit>
#include <QWidget>

struct SceneRectData {
    double x = 0.0;
    double y = 0.0;
    double width = 0.0;
    double height = 0.0;
};

struct ScenePointData {
    double x = 0.0;
    double y = 0.0;
};

struct SceneShadowData {
    QColor color;
    double offsetX = 0.0;
    double offsetY = 0.0;
    double blurRadius = 0.0;
};

struct SceneNodeData {
    int depth = 0;
    QString id;
    QString type;
    QString name;
    QJsonObject params;
    QJsonObject style;
    bool hasBounds = false;
    SceneRectData bounds;
};

struct SceneCanvasItemData {
    QString nodeId;
    QString kind;
    double opacity = 1.0;
    QString blendMode;
    bool hasBounds = false;
    SceneRectData bounds;
    QColor fill;
    double cornerRadius = 0.0;
    bool hasOrigin = false;
    ScenePointData origin;
    QList<ScenePointData> points;
    bool closed = true;
    QString text;
    double fontSize = 12.0;
    QString fontFamily;
    QString imageRef;
    double blurRadius = 0.0;
    bool hasShadow = false;
    SceneShadowData shadow;
};

struct SceneDocumentData {
    QString name;
    double width = 0.0;
    double height = 0.0;
    QColor background = QColor("#f5f1e8");
    QString sourcePath;
    QString selectedNodeId;
    QList<SceneNodeData> nodes;
    QList<SceneCanvasItemData> renderItems;
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
  QRectF canvasRectForWidget() const;
  QPointF mapScenePoint(const ScenePointData& point, const QRectF& canvasRect) const;
  QRectF mapSceneRect(const SceneRectData& rect, const QRectF& canvasRect) const;
  SceneDocumentData scene_;
  SceneNodeData selectedNode_;
};

class MainWindow : public QMainWindow {
  Q_OBJECT

public:
  explicit MainWindow(const QString& scenePath, QWidget* parent = nullptr);

private slots:
  void openSceneDialog();
  void reloadScene();
  void exportPngDialog();
  void applyNodeEdits();
  void handleTreeSelectionChanged();

private:
  void buildUi();
  void buildMenus();
  bool loadScene(const QString& scenePath);
  void populateTree();
  void populateTreeNode(QTreeWidgetItem* parentItem, const SceneNodeData& node);
  void updateInspector(const SceneNodeData& node);
  void populateInspectorFields(const SceneNodeData& node);
  void updateWindowTitle();
  bool exportSceneToPng(const QString& outputPath);
  bool applyNodePropertyEdits(const QString& nodeId, const QString& newName,
                              const QString& textValue, const QString& fillValue);
  bool loadSceneFromEditorCli(const QString& scenePath);
  bool loadSceneFromRawJson(const QString& scenePath);
  QTreeWidgetItem* findTreeItemByNodeId(const QString& nodeId) const;
  QString editorCliPath() const;
  QString objectToPrettyJson(const QJsonObject& object) const;

  SceneDocumentData scene_;
  QMap<QString, SceneNodeData> nodeIndex_;
  QTreeWidget* hierarchyTree_ = nullptr;
  QLineEdit* nameEdit_ = nullptr;
  QPlainTextEdit* textEdit_ = nullptr;
  QLineEdit* fillEdit_ = nullptr;
  QPushButton* applyEditsButton_ = nullptr;
  QTextEdit* inspectorText_ = nullptr;
  CanvasWidget* canvas_ = nullptr;
};
