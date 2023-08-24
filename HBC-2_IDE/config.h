#ifndef CONFIG_H
#define CONFIG_H

#include <QStandardPaths>
#include <QDialog>
#include <QHBoxLayout>
#include <QVBoxLayout>
#include <QCheckBox>
#include <QLineEdit>
#include <QLabel>
#include <QGroupBox>

struct ConfigurationSettings
{
    bool startEmulatorPaused = true;
    bool monitorPlugged = true;
    QString defaultProjectsPath = QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation) + "/hbc2_projects";
};

class ConfigManager
{
    static ConfigManager *m_singleton;

    public:
        static ConfigManager* getInstance();
        ~ConfigManager();

        void setStartEmulatorPaused(bool paused);
        void setMonitorPlugged(bool plugged);
        bool setDefaultProjectsPath(QString defaultPath);

        bool getStartEmulatorPaused();
        bool getMonitorPlugged();
        QString getDefaultProjectsPath();

    private:
        ConfigManager();
        bool saveConfigFile();

        ConfigurationSettings m_settings;
};

class SettingsDialog : public QDialog
{
    Q_OBJECT

    public:
        SettingsDialog(ConfigManager *configManager, QWidget *parent = nullptr);

    private:
        QLineEdit *m_defaultPathLineEdit;

        QCheckBox *m_startPausedCheckBox;
        QCheckBox *m_plugMonitorCheckBox;

        ConfigManager *m_configManager;
};

#endif // CONFIG_H
