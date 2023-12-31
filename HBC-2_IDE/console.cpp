#include "console.h"

#include <QScrollBar>

Console::Console(QWidget *parent) : QTextEdit(parent)
{
    setMinimumHeight(CONSOLE_HEIGHT);
    setMaximumHeight(CONSOLE_HEIGHT);

    setReadOnly(true);
}

void Console::log(QString line)
{
    m_lock.lock();
    time = QTime::currentTime();

    append("[" + time.toString("hh:mm:ss") + "]: " + line + "");
    m_lock.unlock();

    ensureCursorVisible();
}

void Console::log(std::string line)
{
    log(QString::fromStdString(line));
}

void Console::log(const char* line)
{
    log(QString::fromUtf8(line));
}

void Console::log()
{
    returnLine();
}

void Console::returnLine()
{
    m_lock.lock();
    append("");
    m_lock.unlock();

    ensureCursorVisible();
}
