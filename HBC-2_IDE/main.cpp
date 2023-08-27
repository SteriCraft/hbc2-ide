/** \mainpage
 * Home Brew Computer 2 - Integrated Development Environment
 *
 * <a href="https://github.com/SteriCraft/hbc2-ide">Github repository of the project</a><br>
 * <a href="https://www.gnu.org/licenses/gpl-3.0.txt">Licence information</a>
 *
 * \version Version 0.1
 * \author Gianni Leclercq
 * \date 27/08/2023
 */
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
    //a.setStyle("fusion");
    //a.setStyle(QStyleFactory::create("fusion"));

    MainWindow w;

    w.show();

    return a.exec();
}
