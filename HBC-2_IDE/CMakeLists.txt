cmake_minimum_required(VERSION 3.5)

project(HBC-2_IDE VERSION 0.1 LANGUAGES CXX)

set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(QT NAMES Qt6 Qt5 REQUIRED COMPONENTS Widgets Xml OpenGLWidgets)
find_package(Qt${QT_VERSION_MAJOR} COMPONENTS LinguistTools REQUIRED COMPONENTS Widgets Xml OpenGLWidgets)
find_package(OpenGL REQUIRED)
qt_create_translation(QM_FILES ${CMAKE_SOURCE_DIR} fr_lang.ts)

set(PROJECT_SOURCES
        main.cpp
        mainwindow.cpp
        mainwindow.h
        customizedEditor.cpp
        customizedEditor.h
        fileManager.cpp
        fileManager.h
        projectManager.cpp
        projectManager.h
        console.cpp
        console.h
        assembler.cpp
        assembler.h
        token.cpp
        token.h
        chunks.cpp
        chunks.h
        commands.cpp
        commands.h
        qhexedit.cpp
        qhexedit.h
        binaryExplorer.cpp
        binaryExplorer.h
        emulator.cpp
        emulator.h
        cpu.cpp
        cpu.h
        motherboard.cpp
        motherboard.h
        ram.cpp
        ram.h
        #monitor.cpp
        #monitor.h
        cpuDetails.h
        mainwindow.ui
        res.qrc
)

if(${QT_VERSION_MAJOR} GREATER_EQUAL 6)
    qt_add_executable(HBC-2_IDE
        MANUAL_FINALIZATION
        ${PROJECT_SOURCES}
        ${QM_FILES}
        icon.o
    )
# Define target properties for Android with Qt 6 as:
#    set_property(TARGET HBC-2_IDE APPEND PROPERTY QT_ANDROID_PACKAGE_SOURCE_DIR
#                 ${CMAKE_CURRENT_SOURCE_DIR}/android)
# For more information, see https://doc.qt.io/qt-6/qt-add-executable.html#target-creation
else()
    if(ANDROID)
        add_library(HBC-2_IDE SHARED
            ${PROJECT_SOURCES}
        )
# Define properties for Android with Qt 5 after find_package() calls as:
#    set(ANDROID_PACKAGE_SOURCE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/android")
    else()
        add_executable(HBC-2_IDE
            ${PROJECT_SOURCES}
        )
    endif()
endif()

target_link_libraries(HBC-2_IDE PRIVATE
    Qt${QT_VERSION_MAJOR}::Widgets Qt6::Xml Qt6::OpenGLWidgets Qt6::OpenGL ${OPENGL_LIBRARIES}
)

set_target_properties(HBC-2_IDE PROPERTIES
    MACOSX_BUNDLE_GUI_IDENTIFIER my.example.com
    MACOSX_BUNDLE_BUNDLE_VERSION ${PROJECT_VERSION}
    MACOSX_BUNDLE_SHORT_VERSION_STRING ${PROJECT_VERSION_MAJOR}.${PROJECT_VERSION_MINOR}
    MACOSX_BUNDLE TRUE
    WIN32_EXECUTABLE TRUE
)

install(TARGETS HBC-2_IDE
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR})

if(QT_VERSION_MAJOR EQUAL 6)
    qt_finalize_executable(HBC-2_IDE)
endif()
