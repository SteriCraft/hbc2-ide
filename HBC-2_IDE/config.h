#ifndef CONFIG_H
#define CONFIG_H

/*!
 * \file config.h
 * \brief IDE settings management
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
#include <QStandardPaths>
#include <QDialog>
#include <QHBoxLayout>
#include <QVBoxLayout>
#include <QCheckBox>
#include <QLineEdit>
#include <QLabel>
#include <QGroupBox>
#include <QPushButton>
#include <QFileDialog>
#include <QMessageBox>

/*!
 * \brief Stores the IDE settings on runtime.
 */
struct ConfigurationSettings
{
    // Emulator settings
    bool startEmulatorPaused = true; //!< Sets if the emulator starts paused
    bool monitorPlugged = true; //!< Sets if the emulator starts with the monitor plugged in
    bool dismissReassemblyWarnings = false; //!< Sets if warnings are thrown when trying to run a project which was modified or not yet assembled

    // IDE settings
    QString defaultProjectsPath = QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation) + "/hbc2_projects"; //!< Stores the default path for new projects
};

/*!
 * \brief Singleton of the configuration manager, that handles IDE settings
 *
 * Reads and writes settings in the configuration file.<br>
 * The configuration file is stored in the system's default app config directory.<br>
 *
 * For example, on Linux this file can be found here: <i>/home/<username>/.config/HBC-2_IDE/config.cfg</i>
 */
class ConfigManager
{
    static ConfigManager *m_singleton;

    public:
        /*!
         * <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
         */
        static ConfigManager* getInstance();
        ~ConfigManager();

        // SETTERS
        // Emulator settings
        /*!
         * \param paused Desired behaviour for the emulator on run command
         */
        void setStartEmulatorPaused(bool paused);

        /*!
         * \param plugged Desired behaviour for the monitor on emulator start
         */
        void setMonitorPlugged(bool plugged);

        /*!
         * \param dismiss Desired reassembly warnings
         */
        void setDismissReassemblyWarnings(bool dismiss);

        // IDE settings

        /*!
         * \param defaultPath Desired new default path for new projects
         */
        bool setDefaultProjectsPath(QString defaultPath);

        // GETTERS
        // Emulator settings
        /*!
         * \return startEmulatorPaused
         */
        bool getStartEmulatorPaused();

        /*!
         * \return <b>true</b> if the emulator starts with the monitor plugged in
         */
        bool getMonitorPlugged();

        /*!
         * \return <b>true</b> if warnings on emulator run with non assembled project are dismissed
         */
        bool getDismissReassemblyWarnings();

        // IDE settings
        /*!
         * \return default path for new projects
         */
        QString getDefaultProjectsPath();

    private:
        ConfigManager();
        bool saveConfigFile();

        ConfigurationSettings m_settings;
};

/*!
 * \brief QDialog to modify IDE settings
 *
 * Uses ConfigurationManager to read and modify settings.
 */
class SettingsDialog : public QDialog
{
    Q_OBJECT

    public:
        /*!
         * \param configManager Pointer to ConfigManager
         * \param parent Pointer to the QWidget parent
         */
        SettingsDialog(ConfigManager *configManager, QWidget *parent = nullptr);

    private slots:
        void startPausedChanged();
        void plugMonitorChanged();
        void dismissReassemblyWarningsChanged();
        void browseProjectsPathClicked();
        void defaultProjectsPathChanged();

    private:
        QLineEdit *m_defaultProjectsPathLineEdit;

        QCheckBox *m_startPausedCheckBox;
        QCheckBox *m_plugMonitorCheckBox;
        QCheckBox *m_dismissReassemblyWarningsCheckBox;

        ConfigManager *m_configManager;
};

#endif // CONFIG_H
