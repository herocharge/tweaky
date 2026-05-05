cmake_minimum_required(VERSION 3.21)

project(tweaky_editor_qt_shell LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

find_package(Qt6 REQUIRED COMPONENTS Widgets)

qt_add_executable(tweaky-editor-qt
  src/main.cpp
  src/MainWindow.cpp
  src/MainWindow.h
)

target_link_libraries(tweaky-editor-qt PRIVATE Qt6::Widgets)
