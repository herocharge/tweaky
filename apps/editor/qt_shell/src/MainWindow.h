#pragma once

#include <QColor>
#include <QJsonArray>
#include <QJsonObject>
#include <QMainWindow>
#include <QMap>
#include <QRectF>
#include <QTimer>
#include <QDoubleSpinBox>
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
    double positionX = 0.0;
    double positionY = 0.0;
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

signals:
  void nodePicked(const QString& nodeId);

protected:
  void paintEvent(QPaintEvent* event) override;
  void mousePressEvent(QMouseEvent* event) override;

private:
  QRectF canvasRectForWidget() const;
  QPointF mapScenePoint(const ScenePointData& point, const QRectF& canvasRect) const;
  QRectF mapSceneRect(const SceneRectData& rect, const QRectF& canvasRect) const;
  QString pickNodeAt(const QPointF& widgetPoint) const;
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
  void scheduleAutoApply();
  void handleCanvasNodePicked(const QString& nodeId);
  void handleTreeSelectionChanged();

private:
  void buildUi();
  void buildMenus();
  bool loadScene(const QString& scenePath);
  void refreshUiAfterSceneLoad(const QString& statusMessage);
  void populateTree();
  void populateTreeNode(QTreeWidgetItem* parentItem, const SceneNodeData& node);
  void updateInspector(const SceneNodeData& node);
  void populateInspectorFields(const SceneNodeData& node);
  bool inspectorJsonIsValid(QString* errorMessage = nullptr) const;
  void updateWindowTitle();
  bool exportSceneToPng(const QString& outputPath);
  bool applyNodePropertyEdits(const QString& nodeId, const QString& newName, double x, double y,
                              const QString& paramsJson, const QString& styleJson);
  bool loadSceneFromEditorCli(const QString& scenePath,
                              const QStringList& extraArgs = QStringList());
  bool loadSceneFromRawJson(const QString& scenePath);
  QTreeWidgetItem* findTreeItemByNodeId(const QString& nodeId) const;
  QString editorCliPath() const;
  QString objectToPrettyJson(const QJsonObject& object) const;

  SceneDocumentData scene_;
  QMap<QString, SceneNodeData> nodeIndex_;
  QTreeWidget* hierarchyTree_ = nullptr;
  QLineEdit* nameEdit_ = nullptr;
  QDoubleSpinBox* xSpin_ = nullptr;
  QDoubleSpinBox* ySpin_ = nullptr;
  QPlainTextEdit* paramsEdit_ = nullptr;
  QPlainTextEdit* styleEdit_ = nullptr;
  QPushButton* applyEditsButton_ = nullptr;
  QTextEdit* inspectorText_ = nullptr;
  CanvasWidget* canvas_ = nullptr;
  QTimer* autoApplyTimer_ = nullptr;
  bool suppressInspectorSignals_ = false;
};
