#ifndef CONSOLE_H
#define CONSOLE_H

/*!
 * \file console.h
 * \brief Console widget for the IDE
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QTime>
#include <QTextEdit>
#include <QMutex>

/*!
 * \class Console
 * \brief A console widget to show events and provide information to the user
 *
 * Prints time on each log.<br>
 * Thread safe and handles std::string, char* and QString.
 */
class Console : public QTextEdit
{
    Q_OBJECT

    public:
        /*!
         * Constructs the console widget object
         *
         * \param parent Pointer to the parent QWidget
         */
        Console(QWidget *parent);

        /*!
         * Logs a string of character and returns line
         *
         * \param line QString to prompt
         */
        void log(QString line);

        /*!
         * Logs a string of character and returns line
         *
         * \param line std::string to prompt
         */
        void log(std::string line);

        /*!
         * Logs a string of character and returns line
         *
         * \param line String to prompt
         */
        void log(const char* line);

        /*!
         * Returns line (no argument version for safety)
         */
        void log();

        void returnLine();

    private:
        static constexpr int CONSOLE_HEIGHT = 200;

        QTime time;

        QMutex m_lock;
};

#endif // CONSOLE_H
