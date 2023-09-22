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
#include <QTableWidget>
#include <QKeySequenceEdit>

/*!
 * \brief A customized KeySequenceEdit widget that limits to one KeySequence only
 */
class CustomKeySequenceEdit : public QKeySequenceEdit
{
    Q_OBJECT

    public:
        explicit CustomKeySequenceEdit(QWidget *parent = nullptr);
        ~CustomKeySequenceEdit();

    protected:
        void keyPressEvent(QKeyEvent *pEvent);
};

/*!
 * \brief Holds default configuration
 */
namespace Configuration
{
    static constexpr int SHORTCUTS_COUNT = 19;

    enum class Command { NEW_PROJECT = 0, NEW_FILE, OPEN_PROJECT, OPEN_FILE, SAVE_CURRENT_FILE, SAVE_ALL, CLOSE_CURRENT_PROJECT,
                         CLOSE_CURRENT_FILE, CLOSE_ALL_FILES, QUIT, ASSEMBLE, SHOW_BIN_OUTPUT, SHOW_DISASSEMBLY, SHOW_CPU_STATE,
                         RUN_EMULATOR, STEP_EMULATOR, PAUSE_EMULATOR, STOP_EMULATOR, TOGGLE_BREAKPOINT };

    const QString commandStrArr[] = { "Create a new project", "Create a new file",
                                      "Open a project", "Open a file",
                                      "Save the current file", "Save all files",
                                      "Close the current project", "Close the current file",
                                      "Close all files", "Quit",
                                      "Assemble the current project", "Show the binary output (RAM / EEPROM)",
                                      "Show the disassembly viewer", "Show the CPU State Viewer",
                                      "Run the emulator", "Step the emulator",
                                      "Pause the emulator", "Stop the emulator",
                                      "Toggle breakpoint" };

    /*!
 * \brief Stores the IDE settings on runtime
 */
    struct Settings
    {
        // Editor settings
        unsigned int tabSize = 4; //!< Number of spaces representing a tab
        QString defaultProjectsPath = QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation) + "/hbc2_projects"; //!< Stores the default path for new projects

        std::map<Configuration::Command, QKeySequence> shortcutsMap = {
            {Configuration::Command::NEW_PROJECT,           QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_N)},
            {Configuration::Command::NEW_FILE,              QKeySequence(Qt::CTRL | Qt::Key_N)},
            {Configuration::Command::OPEN_PROJECT,          QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_O)},
            {Configuration::Command::OPEN_FILE,             QKeySequence(Qt::CTRL | Qt::Key_O)},
            {Configuration::Command::SAVE_CURRENT_FILE,     QKeySequence(Qt::CTRL | Qt::Key_S)},
            {Configuration::Command::SAVE_ALL,              QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_S)},
            {Configuration::Command::CLOSE_CURRENT_PROJECT, QKeySequence()},
            {Configuration::Command::CLOSE_CURRENT_FILE,    QKeySequence(Qt::CTRL | Qt::Key_W)},
            {Configuration::Command::CLOSE_ALL_FILES,       QKeySequence(Qt::CTRL | Qt::SHIFT | Qt::Key_W)},
            {Configuration::Command::QUIT,                  QKeySequence(Qt::CTRL | Qt::Key_Q)},
            {Configuration::Command::ASSEMBLE,              QKeySequence(Qt::Key_F5)},
            {Configuration::Command::SHOW_BIN_OUTPUT,       QKeySequence(Qt::Key_F6)},
            {Configuration::Command::SHOW_DISASSEMBLY,      QKeySequence(Qt::Key_F7)},
            {Configuration::Command::SHOW_CPU_STATE,        QKeySequence(Qt::Key_F8)},
            {Configuration::Command::RUN_EMULATOR,          QKeySequence(Qt::Key_F9)},
            {Configuration::Command::STEP_EMULATOR,         QKeySequence(Qt::Key_F10)},
            {Configuration::Command::PAUSE_EMULATOR,        QKeySequence(Qt::Key_F11)},
            {Configuration::Command::STOP_EMULATOR,         QKeySequence(Qt::Key_F12)},
            {Configuration::Command::TOGGLE_BREAKPOINT,     QKeySequence(Qt::Key_Pause)}
        };

        // Assembler settings
        bool ramAsDefaultMemoryTarget = true; //<! Sets if the code will be assembled in the RAM by default

        // Emulator settings
        bool startEmulatorPaused = true; //!< Sets if the emulator starts paused
        bool monitorPlugged = true; //!< Sets if the emulator starts with the HbcMonitor plugged in
        bool rtcPlugged = true; //!< Sets if the emulator starts with the HbcRealTimeClock plugged in
        bool keyboardPlugged = true; //!< Sets if the emulator starts with the HbcKeyboard plugged in
        bool eepromPlugged = false; //!< Sets if the emulator starts with the HbcEeprom plugged in
        bool dismissReassemblyWarnings = false; //!< Sets if warnings are thrown when trying to run a project which was modified or not yet assembled
        unsigned int pixelScale = 4; //!< Sets the size of a pixel in the HbcMonitor

        // CPU state viewer settings
        bool openCpuStateViewerOnEmulatorPaused = true;
        bool openCpuStateViewerOnEmulatorStopped = true;

        // Binary viewer settings
        bool openBinaryViewerOnAssembly = true; //!< Sets if the binary viewer is automatically opened after assembling a project
        bool openBinaryViewerOnEmulatorPaused = true; //!< Sets if the binary viewer is automatically opened after the emulator was paused
        bool openBinaryViewerOnEmulatorStopped = true; //!< Sets if the binary viewer is automatically opened after the emulator was stopped

        // Disassembly viewer settings
        bool openDisassemblyViewerOnAssembly = false; //!< Sets if the disassembly viewer is automatically opened after assembling a project
        bool openDisassemblyViewerOnEmulatorPaused = false; //!< Sets if the disassemblyviewer is automatically opened after the emulator was paused
        bool openDisassemblyViewerOnEmulatorStopped = false; //!< Sets if the disassemblyviewer is automatically opened after the emulator was stopped
    };
}

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

        void resetShortcut(Configuration::Command associatedCommand);
        void resetAllShortcuts();
        void resetSettings();


        // ==== SETTERS ====
        // Editor settings
        /*!
         * \brief Sets the number of spaces representing a tab
         */
        void setTabSize(unsigned int nbOfSpaces);

        /*!
         * \param defaultPath Desired new default path for new projects
         */
        bool setDefaultProjectsPath(QString defaultPath);

        // Assembler settings
        /*!
         * \brief Sets if the code will be assembled in the RAM
         */
        void setRamAsDefaultMemoryTarget(bool ramAsDefault);

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

        /*!
         * \brief Sets the scale of a pixel in HbcMonitor
         */
        void setPixelScale(unsigned int scale);

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

        // Disassembly viewer settings
        /*!
         * \param enable Desired behaviour for the DisassemblyViewer on project assembly
         */
        void setOpenDisassemblyViewerOnAssembly(bool enable);

        /*!
         * \param enable Desired behaviour for the DisassemblyViewer when the emulator is paused
         */
        void setOpenDisassemblyViewerOnEmulatorPaused(bool enable);

        /*!
         * \param enable Desired behaviour for the DisassemblyViewer when the emulator is stopped
         */
        void setOpenDisassemblyViewerOnEmulatorStopped(bool enable);


        // ==== GETTERS ====
        // Editor settings
        /*!
         * \return the number of spaces representing a tab char
         */
        unsigned int getTabSize();

        QString getDefaultProjectsPath();

        std::map<Configuration::Command, QKeySequence>& getShortcutsMap();

        // Assembler settings
        /*!
         * \return <b>true</b> if the code must be assembled in the RAM by default
         */
        bool getRamAsDefaultMemoryTarget();

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

        /*!
         * \return the scale of a pixel in HbcMonitor
         */
        unsigned int getPixelScale();

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

        // Disassembly viewer settings
        /*!
         * \return <b>true</b> if the DisassemblyViewer opens on project assembly
         */
        bool getOpenDisassemblyViewerOnAssembly();

        /*!
         * \return <b>true</b> if the DisassemblyViewer opens when the emulator is paused
         */
        bool getOpenDisassemblyViewerOnEmulatorPaused();

        /*!
         * \return <b>true</b> if the DisassemblyViewer opens when the emulator is stopped
         */
        bool getOpenDisassemblyViewerOnEmulatorStopped();

    private:
        ConfigManager();
        bool saveRecentProjects();
        bool saveConfigFile();
        void flushRecentProjects();

        Configuration::Settings *m_settings;
        QList<QString> m_recentProjects;
};

class MainWindow;

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
        SettingsDialog(ConfigManager *configManager, MainWindow *parent);

    private slots:
        void menuSelected(int tabIndex);

        // Editor
        void tabSizeChanged(int nbOfSpaces);
        void browseProjectsPathClicked();
        void defaultProjectsPathChanged(QString newPath);
        void shortcutSelected();
        void saveButtonClicked();
        void resetSelectedShortcut();
        void resetAllShortcuts();

        // Assembler
        void ramAsDefaultMemoryTargetChanged();

        // Emulator
        void startPausedChanged();
        void plugMonitorChanged();
        void plugRTCChanged();
        void plugKeyboardChanged();
        void plugEepromChanged();
        void dismissReassemblyWarningsChanged();
        void pixelScaleChanged(int scale);

        // CpuStateViewer
        void openCpuStateViewerOnEmulatorPausedChanged();
        void openCpuStateViewerOnEmulatorStoppedChanged();

        // BinaryViewer
        void openBinaryViewerOnAssemblyChanged();
        void openBinaryViewerOnEmulatorPausedChanged();
        void openBinaryViewerOnEmulatorStoppedChanged();

        // DisassemblyViewer
        void openDisassemblyViewerOnAssemblyChanged();
        void openDisassemblyViewerOnEmulatorPausedChanged();
        void openDisassemblyViewerOnEmulatorStoppedChanged();

        // Others
        void resetAllAction();

    private:
        static constexpr int DIALOG_WIDTH = 800;
        static constexpr int DIALOG_HEIGHT = 400;
        static constexpr int MENU_LIST_WIDTH = 160;
        static constexpr int ITEM_HEIGHT = 50;
        static constexpr int SPACING = 20;

        void initMenu();
        void setWindowSettings();
        void initEditorSettingsLayout();
        void initAssemblerSettingsLayout();
        void initEmulatorSettingsLayout();
        void initCpuStateViewerSettingsLayout();
        void initBinaryViewerSettingsLayout();
        void initDisassemblyViewerSettingsLayout();
        void addMenu(QListWidgetItem *newMenuItem, QWidget *newMenuWidget);

        void updateWidgets();

        QListWidget *m_menuList;
        QStackedWidget *m_menuWidgets;
        std::vector<QWidget*> m_menuWidgetsList;

        // Editor page widgets
        // General tab
        QPushButton *m_browseProjectsPath;
        QLineEdit *m_defaultProjectsPathLineEdit;
        QSpinBox *m_tabSizeSpinBox;
        QVBoxLayout *m_editorSettingsGeneralTabLayout;
        QWidget *m_editorSettingsGeneralTabWidget;
        // Keyboard tab
        QTableWidget *m_keyboardShortcutsTableWidget;
        CustomKeySequenceEdit *m_keySequenceEdit;
        QPushButton *m_keyboardShortcutsSaveButton;
        QPushButton *m_keyboardShortcutsResetButton;
        QVBoxLayout *m_editorSettingsKeyboardTabLayout;
        QWidget *m_editorSettingsKeyboardTabWidget;

        QTabWidget *m_editorSettingsTabWidget;
        QVBoxLayout *m_editorSettingsPageLayout;
        QWidget *m_editorSettingsPageWidget;
        QListWidgetItem *m_editorSettingsItem;

        // Assembler page widgets
        QCheckBox *m_ramAsDefaultMemoryTargetCheckBox;
        QVBoxLayout *m_assemblerSettingsGeneralTabLayout;
        QWidget *m_assemblerSettingsGeneralTabWidget;

        QTabWidget *m_assemblerSettingsTabWidget;
        QVBoxLayout *m_assemblerSettingsPageLayout;
        QWidget *m_assemblerSettingsPageWidget;
        QListWidgetItem *m_assemblerSettingsItem;

        // Emulator page widgets
        // General tab
        QCheckBox *m_startPausedCheckBox;
        QCheckBox *m_dismissReassemblyWarningsCheckBox;
        QVBoxLayout *m_emulatorSettingsGeneralTabLayout;
        QWidget *m_emulatorSettingsGeneralTabWidget;
        // Monitor tab
        QCheckBox *m_plugMonitorCheckBox;
        QSpinBox *m_pixelScaleSpinBox;
        QVBoxLayout *m_emulatorSettingsMonitorTabLayout;
        QWidget *m_emulatorSettingsMonitorTabWidget;
        // RTC tab
        QCheckBox *m_plugRTCCheckBox;
        QVBoxLayout *m_emulatorSettingsRtcTabLayout;
        QWidget *m_emulatorSettingsRtcTabWidget;
        // Keyboard tab
        QCheckBox *m_plugKeyboardCheckBox;
        QVBoxLayout *m_emulatorSettingsKeyboardTabLayout;
        QWidget *m_emulatorSettingsKeyboardTabWidget;
        // EEPROM tab
        QCheckBox *m_plugEepromCheckBox;
        QVBoxLayout *m_emulatorSettingsEepromTabLayout;
        QWidget *m_emulatorSettingsEepromTabWidget;

        QTabWidget *m_emulatorSettingsTabWidget;
        QVBoxLayout *m_emulatorSettingsPageLayout;
        QWidget *m_emulatorSettingsPageWidget;
        QListWidgetItem *m_emulatorSettingsItem;

        // Cpu state viewer page widgets
        QCheckBox *m_openCpuStateViewerOnEmulatorPausedCheckBox;
        QCheckBox *m_openCpuStateViewerOnEmulatorStoppedCheckBox;
        QVBoxLayout *m_cpuStateViewerSettingsGeneralTabLayout;
        QWidget *m_cpuStateViewerSettingsGeneralTabWidget;

        QTabWidget *m_cpuStateViewerSettingsTabWidget;
        QVBoxLayout *m_cpuStateViewerSettingsPageLayout;
        QWidget *m_cpuStateViewerSettingsPageWidget;
        QListWidgetItem *m_cpuStateViewerSettingsItem;

        // Binary viewer tab widgets
        QCheckBox *m_openBinaryViewerOnAssemblyCheckBox;
        QCheckBox *m_openBinaryViewerOnEmulatorPausedCheckBox;
        QCheckBox *m_openBinaryViewerOnEmulatorStoppedCheckBox;
        QVBoxLayout *m_binaryViewerSettingsGeneralTabLayout;
        QWidget *m_binaryViewerSettingsGeneralTabWidget;

        QTabWidget *m_binaryViewerSettingsTabWidget;
        QVBoxLayout *m_binaryViewerSettingsPageLayout;
        QWidget *m_binaryViewerSettingsPageWidget;
        QListWidgetItem *m_binaryViewerSettingsItem;

        // Disassembly viewer tab widgets
        QCheckBox *m_openDisassemblyViewerOnAssemblyCheckBox;
        QCheckBox *m_openDisassemblyViewerOnEmulatorPausedCheckBox;
        QCheckBox *m_openDisassemblyViewerOnEmulatorStoppedCheckBox;
        QVBoxLayout *m_disassemblyViewerSettingsGeneralTabLayout;
        QWidget *m_disassemblyViewerSettingsGeneralTabWidget;

        QTabWidget *m_disassemblyViewerSettingsTabWidget;
        QVBoxLayout *m_disassemblyViewerSettingsPageLayout;
        QWidget *m_disassemblyViewerSettingsPageWidget;
        QListWidgetItem *m_disassemblyViewerSettingsItem;

        QPushButton *m_resetAllButton;
        QPushButton *m_closeButton;

        ConfigManager *m_configManager;
};

#endif // CONFIG_H
