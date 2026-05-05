#include <QApplication>

#include "MainWindow.h"

int main(int argc, char* argv[]) {
  QApplication app(argc, argv);

  QString scenePath = argc > 1 ? QString::fromLocal8Bit(argv[1])
                               : QStringLiteral("examples/basic_poster.vsd.json");

  MainWindow window(scenePath);
  window.show();

  return app.exec();
}
