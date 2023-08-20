#include <QApplication>
#include <QTranslator>
#include <QStyleFactory>

#include "mainwindow.h"

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

    //a.installTranslator(&translator);
    //a.setStyle("Fusion");
    a.setStyle(QStyleFactory::create("Fusion"));

    MainWindow w;

    w.show();

    return a.exec();
}
