
#ifndef CONSOLE_H
#define CONSOLE_H

#include <QTime>
#include <QTextEdit>

#define CONSOLE_HEIGHT 200

class Console : public QTextEdit
{
    public:
        Console(QWidget *parent);

        void log(QString line);
        void log(std::string line);
        void log(const char* line);

        void log(); // Just in case, returns to the line
        void returnLine();

    private:
        QTime time;
};

#endif // CONSOLE_H
