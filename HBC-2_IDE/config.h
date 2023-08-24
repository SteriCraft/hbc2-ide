#ifndef CONFIG_H
#define CONFIG_H

#include <QStandardPaths>

struct ConfigurationSettings
{
    bool startEmulatorPaused = true;
    bool monitorPlugged = true;
    QString defaultProjectsPath = QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation) + "/hbc2_projects";
};

class Configuration
{
    static Configuration *m_singleton;

    public:
        static Configuration* getInstance();
        ~Configuration();

        void setStartEmulatorPaused(bool paused);
        void setMonitorPlugged(bool plugged);
        bool setDefaultProjectsPath(QString defaultPath);

        bool getStartEmulatorPaused();
        bool getMonitorPlugged();
        QString getDefaultProjectsPath();

    private:
        Configuration();
        bool saveConfigFile();

        ConfigurationSettings m_settings;
};

#endif // CONFIG_H
