#pragma once

#include <QColor>
#include <QCloseEvent>
#include <QJsonArray>
#include <QJsonObject>
#include <QKeyEvent>
#include <QMainWindow>
#include <QMap>
#include <QRectF>
#include <QTemporaryDir>
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

enum class ResizeHandle {
    None = 0,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
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
    QString workingPath;
    bool dirty = false;
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
  void nodeDragPreview(double x, double y);
  void nodeDragCommitted(const QString& nodeId, double x, double y);
  void nodeResizePreview(const QString& nodeId, double x, double y, double width, double height);
  void nodeResizeCommitted(const QString& nodeId, double x, double y, double width, double height);

protected:
  void paintEvent(QPaintEvent* event) override;
  void mousePressEvent(QMouseEvent* event) override;
  void mouseMoveEvent(QMouseEvent* event) override;
  void mouseReleaseEvent(QMouseEvent* event) override;

private:
  QRectF canvasRectForWidget() const;
  QPointF mapScenePoint(const ScenePointData& point, const QRectF& canvasRect) const;
  QRectF mapSceneRect(const SceneRectData& rect, const QRectF& canvasRect) const;
  QPointF activeDragWidgetOffset() const;
  SceneRectData activeResizeSceneRect() const;
  QPointF scenePositionForWidgetPoint(const QPointF& widgetPoint) const;
  QString pickNodeAt(const QPointF& widgetPoint) const;
  bool selectedNodeSupportsResize() const;
  QRectF selectedOutlineRect(const QRectF& canvasRect) const;
  ResizeHandle resizeHandleAt(const QPointF& widgetPoint, const QRectF& canvasRect) const;
  SceneDocumentData scene_;
  SceneNodeData selectedNode_;
  bool dragActive_ = false;
  QString dragNodeId_;
  QPointF dragStartWidgetPos_;
  QPointF dragCurrentWidgetPos_;
  double dragStartSceneX_ = 0.0;
  double dragStartSceneY_ = 0.0;
  bool resizeActive_ = false;
  ResizeHandle activeResizeHandle_ = ResizeHandle::None;
  QString resizeNodeId_;
  QPointF resizeCurrentWidgetPos_;
  SceneRectData resizeStartSceneRect_;
};

class MainWindow : public QMainWindow {
  Q_OBJECT

public:
  explicit MainWindow(const QString& scenePath, QWidget* parent = nullptr);

private slots:
  void openSceneDialog();
  void reloadScene();
  void saveScene();
  void saveSceneAs();
  void exportPngDialog();
  void applyNodeEdits();
  void scheduleAutoApply();
  void handleCanvasNodePicked(const QString& nodeId);
  void handleCanvasNodeDragPreview(double x, double y);
  void handleCanvasNodeDragCommitted(const QString& nodeId, double x, double y);
  void handleCanvasNodeResizePreview(const QString& nodeId, double x, double y,
                                     double width, double height);
  void handleCanvasNodeResizeCommitted(const QString& nodeId, double x, double y,
                                       double width, double height);
  void handleTreeSelectionChanged();

private:
  void closeEvent(QCloseEvent* event) override;
  void keyPressEvent(QKeyEvent* event) override;
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
  void markDirty(bool dirty);
  bool ensureWorkingCopyFromSource(const QString& sourcePath);
  bool saveWorkingCopyToPath(const QString& outputPath);
  bool maybeResolveUnsavedChanges(const QString& actionLabel);
  bool nudgeSelectedNode(double deltaX, double deltaY);
  bool resizeNodeToBounds(const QString& nodeId, double x, double y, double width, double height);
  bool exportSceneToPng(const QString& outputPath);
  bool applyNodePropertyEdits(const QString& nodeId, const QString& newName, double x, double y,
                              const QString& paramsJson, const QString& styleJson);
  bool loadSceneFromEditorCli(const QString& scenePath, const QString& sourcePath = QString(),
                              const QStringList& extraArgs = QStringList());
  bool loadSceneFromRawJson(const QString& scenePath, const QString& sourcePath = QString());
  QTreeWidgetItem* findTreeItemByNodeId(const QString& nodeId) const;
  QString editorCliPath() const;
  QString objectToPrettyJson(const QJsonObject& object) const;

  SceneDocumentData scene_;
  QTemporaryDir workingCopyDirectory_;
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
