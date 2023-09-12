#ifndef CONFIG_H
#define CONFIG_H

/*!
 * \file config.h
 * \brief IDE settings management
 * \author Gianni Leclercq
 * \version 0.1
 * \date 29/08/2023
 */
#include <QStandardPaths>
#include <QDialog>
#include <QHBoxLayout>
#include <QVBoxLayout>
#include <QCheckBox>
#include <QLineEdit>
#include <QLabel>
#include <QPushButton>
#include <QFileDialog>
#include <QSpinBox>
#include <QTabWidget>
#include <QListWidget>
#include <QStackedWidget>
#include <QMessageBox>

/*!
 * \brief Stores the IDE settings on runtime.
 */
struct ConfigurationSettings
{
    // Editor settings
    unsigned int tabSize = 4; // Number of spaces representing a tab

    // Assembler settings
    bool ramAsDefaultMemoryTarget = true; //<! Sets if the code will be assembled in the RAM by default

    // CPU state viewer settings
    bool openCpuStateViewerOnEmulatorPaused = true;
    bool openCpuStateViewerOnEmulatorStopped = true;

    // Binary viewer settings
    bool openBinaryViewerOnAssembly = true; //!< Sets if the viewer is automatically opened after assembling a project
    bool openBinaryViewerOnEmulatorPaused = true; //!< Sets if the viewer is automatically opened after the emulator was paused
    bool openBinaryViewerOnEmulatorStopped = true; //!< Sets if the viewer is automatically opened after the emulator was stopped

    // Emulator settings
    bool startEmulatorPaused = true; //!< Sets if the emulator starts paused
    bool monitorPlugged = true; //!< Sets if the emulator starts with the HbcMonitor plugged in
    bool rtcPlugged = true; //!< Sets if the emulator starts with the HbcRealTimeClock plugged in
    bool keyboardPlugged = true; //!< Sets if the emulator starts with the HbcKeyboard plugged in
    bool eepromPlugged = false; //!< Sets if the emulator starts with the HbcEeprom plugged in
    bool dismissReassemblyWarnings = false; //!< Sets if warnings are thrown when trying to run a project which was modified or not yet assembled

    // IDE settings
    QString defaultProjectsPath = QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation) + "/hbc2_projects"; //!< Stores the default path for new projects

    // Recent projects
    QList<QString> recentProjects;
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

        /*!
         * \return a list of recent projects stored in "recentProjects.cfg"
         */
        QList<QString> getRecentProjects();

        /*!
         * \brief Returns <b>false</b> if path is empty
         */
        bool addRecentProject(QString path);

        /*!
         * \brief Clears the list of recent projects
         */
        void clearRecentProjects();


        // ==== SETTERS ====
        // Editor settings
        /*!
         * \brief Sets the number of spaces representing a tab
         */
        void setTabSize(unsigned int nbOfSpaces);

        // Assembler settings
        /*!
         * \brief Sets if the code will be assembled in the RAM
         */
        void setRamAsDefaultMemoryTarget(bool ramAsDefault);

        // CPU state viewer settings
        /*!
         * \param enable Desired behaviour for the CpuStateViewer when the emulator is paused
         */
        void setOpenCpuStateViewerOnEmulatorPaused(bool enable);

        /*!
         * \param enable Desired behaviour for the CpuStateViewer when the emulator is stopped
         */
        void setOpenCpuStateViewerOnEmulatorStopped(bool enable);

        // Binary viewer settings
        /*!
         * \param enable Desired behaviour for the BinaryViewer on project assembly
         */
        void setOpenBinaryViewerOnAssembly(bool enable);

        /*!
         * \param enable Desired behaviour for the BinaryViewer when the emulator is paused
         */
        void setOpenBinaryViewerOnEmulatorPaused(bool enable);

        /*!
         * \param enable Desired behaviour for the BinaryViewer when the emulator is stopped
         */
        void setOpenBinaryViewerOnEmulatorStopped(bool enable);

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
         * \param plugged Desired behaviour for the real-time clock on emulator start
         */
        void setRTCPlugged(bool plugged);

        /*!
         * \param plugged Desired behaviour for the keyboard on emulator start
         */
        void setKeyboardPlugged(bool plugged);

        /*!
         * \param plugged Desired behaviour for the EEPROM on emulator start
         */
        void setEepromPlugged(bool plugged);

        /*!
         * \param dismiss Desired reassembly warnings
         */
        void setDismissReassemblyWarnings(bool dismiss);

        // IDE settings
        /*!
         * \param defaultPath Desired new default path for new projects
         */
        bool setDefaultProjectsPath(QString defaultPath);


        // ==== GETTERS ====
        // Editor settings
        /*!
         * \return the number of spaces representing a tab char
         */
        unsigned int getTabSize();

        // Assembler settings
        /*!
         * \return <b>true</b> if the code must be assembled in the RAM by default
         */
        bool getRamAsDefaultMemoryTarget();

        // Cpu state viewer settings
        /*!
         * \return <b>true</b> if the CpuStateViewer opens when the emulator is paused
         */
        bool getOpenCpuStateViewerOnEmulatorPaused();

        /*!
         * \return <b>true</b> if the CpuStateViewer opens when the emulator is stopped
         */
        bool getOpenCpuStateViewerOnEmulatorStopped();

        // Binary viewer settings
        /*!
         * \return <b>true</b> if the BinaryViewer opens on project assembly
         */
        bool getOpenBinaryViewerOnAssembly();

        /*!
         * \return <b>true</b> if the BinaryViewer opens when the emulator is paused
         */
        bool getOpenBinaryViewerOnEmulatorPaused();

        /*!
         * \return <b>true</b> if the BinaryViewer opens when the emulator is stopped
         */
        bool getOpenBinaryViewerOnEmulatorStopped();

        // Emulator settings
        /*!
         * \return <b>true</b> if the emulator starts paused
         */
        bool getStartEmulatorPaused();

        /*!
         * \return <b>true</b> if the emulator starts with the monitor plugged in
         */
        bool getMonitorPlugged();

        /*!
         * \return <b>true</b> if the emulator starts with the real-time clock plugged in
         */
        bool getRTCPlugged();

        /*!
         * \return <b>true</b> if the emulator starts with the keyboard plugged in
         */
        bool getKeyboardPlugged();

        /*!
         * \return <b>true</b> if the emulator starts with the EEPROM plugged in
         */
        bool getEepromPlugged();

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
        bool saveRecentProjects();
        bool saveConfigFile();
        void flushRecentProjects();

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
        void menuSelected(int tabIndex);

        // Editor
        void tabSizeChanged(int nbOfSpaces);
        void browseProjectsPathClicked();
        void defaultProjectsPathChanged(QString newPath);

        // Assembler
        void ramAsDefaultMemoryTargetChanged();

        // Emulator
        void startPausedChanged();
        void plugMonitorChanged();
        void plugRTCChanged();
        void plugKeyboardChanged();
        void plugEepromChanged();
        void dismissReassemblyWarningsChanged();

        // CpuStateViewer
        void openCpuStateViewerOnEmulatorPausedChanged();
        void openCpuStateViewerOnEmulatorStoppedChanged();

        // BinaryViewer
        void openBinaryViewerOnAssemblyChanged();
        void openBinaryViewerOnEmulatorPausedChanged();
        void openBinaryViewerOnEmulatorStoppedChanged();

    private:
        static constexpr int DIALOG_WIDTH = 640;
        static constexpr int DIALOG_HEIGHT = 350;
        static constexpr int ITEM_HEIGHT = 50;
        static constexpr int SPACING = 20;

        void initMenu();
        void setWindowSettings();
        void initEditorSettingsLayout();
        void initAssemblerSettingsLayout();
        void initEmulatorSettingsLayout();
        void initCpuStateViewerSettingsLayout();
        void initBinaryViewerSettingsLayout();

        void addMenu(QListWidgetItem *newMenuItem, QWidget *newMenuWidget);

        QListWidget *m_menuList;
        QStackedWidget *m_menuWidgets;
        std::vector<QWidget*> m_menuWidgetsList;

        // Editor tab widgets
        QPushButton *m_browseProjectsPath;
        QLineEdit *m_defaultProjectsPathLineEdit;
        QSpinBox *m_tabSizeSpinBox;
        QVBoxLayout *m_editorSettingsLayout;
        QWidget *m_editorSettingsWidget;
        QListWidgetItem *m_editorSettingsItem;

        // Assembler tab widgets
        QCheckBox *m_ramAsDefaultMemoryTargetCheckBox;
        QVBoxLayout *m_assemblerSettingsLayout;
        QWidget *m_assemblerSettingsWidget;
        QListWidgetItem *m_assemblerSettingsItem;

        // Emulator tab widgets
        QCheckBox *m_startPausedCheckBox;
        QCheckBox *m_plugMonitorCheckBox;
        QCheckBox *m_plugRTCCheckBox;
        QCheckBox *m_plugKeyboardCheckBox;
        QCheckBox *m_plugEepromCheckBox;
        QCheckBox *m_dismissReassemblyWarningsCheckBox;
        QVBoxLayout *m_emulatorSettingsLayout;
        QWidget *m_emulatorSettingsWidget;
        QListWidgetItem *m_emulatorSettingsItem;

        // Cpu state viewer tab widgets
        QCheckBox *m_openCpuStateViewerOnEmulatorPausedCheckBox;
        QCheckBox *m_openCpuStateViewerOnEmulatorStoppedCheckBox;
        QVBoxLayout *m_cpuStateViewerSettingsLayout;
        QWidget *m_cpuStateViewerSettingsWidget;
        QListWidgetItem *m_cpuStateViewerSettingsItem;

        // Binary viewer tab widgets
        QCheckBox *m_openBinaryViewerOnAssemblyCheckBox;
        QCheckBox *m_openBinaryViewerOnEmulatorPausedCheckBox;
        QCheckBox *m_openBinaryViewerOnEmulatorStoppedCheckBox;
        QVBoxLayout *m_binaryViewerSettingsLayout;
        QWidget *m_binaryViewerSettingsWidget;
        QListWidgetItem *m_binaryViewerSettingsItem;

        QPushButton *m_closeButton;

        ConfigManager *m_configManager;
};

#endif // CONFIG_H
