cmake_minimum_required(VERSION 3.14)

project(HBC-2_IDE LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_CXX_FLAGS_RELEASE "-O3")

find_package(QT NAMES Qt6 Qt5 REQUIRED COMPONENTS Core Widgets Xml OpenGLWidgets)
find_package(Qt${QT_VERSION_MAJOR} REQUIRED COMPONENTS Core Widgets Xml OpenGLWidgets)
find_package(OpenGL REQUIRED)

#NOT WORKING
#qt_create_translation(QM_FILES ${CMAKE_SOURCE_DIR} fr_lang.ts)

add_executable(HBC-2_IDE
  main.cpp
  assembler.cpp
  assembler.h
  binaryViewer.cpp
  binaryViewer.h
  chunks.cpp
  chunks.h
  commands.cpp
  commands.h
  computerDetails.h
  config.cpp
  config.h
  console.cpp
  console.h
  cpu.cpp
  cpu.h
  cpuStateViewer.cpp
  cpuStateViewer.h
  customizedEditor.cpp
  customizedEditor.h
  emulator.cpp
  emulator.h
  fileManager.cpp
  fileManager.h
  iod.cpp
  iod.h
  mainwindow.cpp
  mainwindow.h
  monitor.cpp
  monitor.h
  motherboard.cpp
  motherboard.h
  peripheral.cpp
  peripheral.h
  projectManager.cpp
  projectManager.h
  qhexedit.cpp
  qhexedit.h
  ram.cpp
  ram.h
  realTimeClock.cpp
  realTimeClock.h
  token.cpp
  token.h
  mainwindow.ui
  res.qrc
)

#Adding executable icon
if (WIN32)
    if(${QT_VERSION_MAJOR} GREATER_EQUAL 6)
        qt_add_executable(HBC-2_IDE MANUAL_FINALIZATION ${PROJECT_SOURCES} ${QM_FILES} icon.o )
    endif()
#elseif (UNIX)
# Linux distributions manage icons differently depending on the desktop environment
# See https://doc.qt.io/qt-6/appicon.html
#endif()
endif()


target_link_libraries(HBC-2_IDE Qt${QT_VERSION_MAJOR}::Core
                                Qt${QT_VERSION_MAJOR}::Widgets
                                Qt${QT_VERSION_MAJOR}::Xml
                                Qt${QT_VERSION_MAJOR}::OpenGLWidgets
                                Qt${QT_VERSION_MAJOR}::OpenGL
                                ${OPENGL_LIBRARIES})

include(GNUInstallDirs)
install(TARGETS HBC-2_IDE
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
