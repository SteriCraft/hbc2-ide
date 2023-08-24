#include "config.h"

#include <QFile>
#include <QDir>
#include <QTextStream>
#include <QDebug>

Configuration* Configuration::m_singleton = nullptr;

// PUBLIC
Configuration* Configuration::getInstance()
{
    if (m_singleton == nullptr)
        m_singleton = new Configuration();

    return m_singleton;
}

Configuration::~Configuration()
{
    saveConfigFile();
    m_singleton = nullptr;
}

void Configuration::setStartEmulatorPaused(bool paused)
{
    m_settings.startEmulatorPaused = paused;
    saveConfigFile();
}

void Configuration::setMonitorPlugged(bool plugged)
{
    m_settings.monitorPlugged = plugged;
    saveConfigFile();
}

bool Configuration::setDefaultProjectsPath(QString defaultPath)
{
    if (defaultPath.isEmpty())
        return false;

    m_settings.defaultProjectsPath = defaultPath;
    saveConfigFile();

    return true;
}

bool Configuration::getStartEmulatorPaused()
{
    return m_settings.startEmulatorPaused;
}

bool Configuration::getMonitorPlugged()
{
    return m_settings.monitorPlugged;
}

QString Configuration::getDefaultProjectsPath()
{
    return m_settings.defaultProjectsPath;
}

// PRIVATE
Configuration::Configuration()
{
    QString configFilePath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/config.cfg");
    QFile configFile(configFilePath);

    if (QFile::exists(configFilePath))
    {
        if (configFile.open(QIODevice::ReadOnly))
        {
            QTextStream in(&configFile);

            while (!in.atEnd())
            {
                QString line = in.readLine();

                int equalCharPos(line.indexOf('='));
                QString key, value;

                if (equalCharPos != -1)
                {
                    key = line.mid(0, equalCharPos);
                    value = line.mid(equalCharPos + 1);

                    if (key == "START_EMULATOR_PAUSED")
                    {
                        m_settings.startEmulatorPaused = (value == "TRUE");
                    }
                    else if (key == "MONITOR_PLUGGED")
                    {
                        m_settings.monitorPlugged = (value == "TRUE");
                    }
                    else if (key == "DEFAULT_PROJECT_PATH")
                    {
                        if (!value.isEmpty())
                            m_settings.defaultProjectsPath = value;

                        QString defaultProjectsDirPath(m_settings.defaultProjectsPath);
                        QDir defaultProjectsDir(defaultProjectsDirPath);

                        if (!defaultProjectsDir.exists())
                        {
                            defaultProjectsDir.mkpath(defaultProjectsDirPath);
                        }
                    }
                }
            }
        }
    }
}

bool Configuration::saveConfigFile()
{
    QString configFilePath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/config.cfg");
    QFile configFile(configFilePath);

    QString configDirectoryPath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation));
    QDir configDirectory(configDirectoryPath);

    if (!configDirectory.exists())
    {
        configDirectory.mkpath(configDirectoryPath);
    }

    if (configFile.open(QIODevice::WriteOnly))
    {
        QTextStream out(&configFile);

        out << "START_EMULATOR_PAUSED=" << (m_settings.startEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "MONITOR_PLUGGED=" << (m_settings.monitorPlugged ? "TRUE" : "FALSE") << "\n";
        out << "DEFAULT_PROJECT_PATH=" << m_settings.defaultProjectsPath << "\n";

        return true;
    }

    return false;
}
