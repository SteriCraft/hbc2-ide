#include <QApplication>
#include <QTranslator>

#include "mainwindow.h"

// === TODOS ===
// -> Check C++ code tips, tricks and conventions
// -> Rename option hasn't been implemented yet
// -> Only show one binary output window at a time
// -> Update its content every assembly run
// -> Prevent instructions without any label before in the same file (currently .include allows that behaviour)
// -> Assembler with debug mode
//    -> .interrupt <8 bits> <label_name>
//    -> .msb, .lsb, .len
// -> Consider limit use of variables in the scope of their definition file (easy), and the ones that follow this file's inclusion (tricky)
// -> Integrate the emulator with a debug mode
// -> Adapt the IDE buttons to include debug mode
// -> Make project static on release
// =============

// === BUGS ===
//
// ============

int main(int argc, char *argv[])
{
    QApplication a(argc, argv);
    QTranslator translator;
    bool translateInFrench(false);

    if (translateInFrench)
    {
        if (!translator.load("fr_lang"))
            qDebug() << "Couldn't load the french translation file";
        else
            qDebug() << "French translation file loaded successfuly";
    }

    a.installTranslator(&translator);
    a.setStyle("fusion");

    MainWindow w;

    w.show();

    return a.exec();
}
