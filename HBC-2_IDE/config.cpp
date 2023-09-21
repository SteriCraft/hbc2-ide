#include "config.h"
#include "qheaderview.h"
#include "mainWindow.h"

#include <QFile>
#include <QDir>
#include <QTextStream>
#include <QDebug>

// ========= CustomKeySequenceEdit class =========
CustomKeySequenceEdit::CustomKeySequenceEdit(QWidget *parent) : QKeySequenceEdit(parent)
{ }

CustomKeySequenceEdit::~CustomKeySequenceEdit()
{ }

void CustomKeySequenceEdit::keyPressEvent(QKeyEvent *pEvent)
{
    QKeySequenceEdit::keyPressEvent(pEvent);

    QKeySequence seq(QKeySequence::fromString(keySequence().toString().split(", ").first()));
    setKeySequence(seq);
}


// ========= ConfigManager class =========
ConfigManager* ConfigManager::m_singleton = nullptr;

// PUBLIC
ConfigManager* ConfigManager::getInstance()
{
    if (m_singleton == nullptr)
        m_singleton = new ConfigManager();

    return m_singleton;
}

ConfigManager::~ConfigManager()
{
    saveConfigFile();
    saveRecentProjects();

    m_singleton = nullptr;
}

QList<QString> ConfigManager::getRecentProjects()
{
    return m_recentProjects;
}

bool ConfigManager::addRecentProject(QString path)
{
    m_recentProjects.push_front(path);

    flushRecentProjects();

    return true;
}

void ConfigManager::clearRecentProjects()
{
    m_recentProjects.clear();
}

void ConfigManager::resetShortcut(Configuration::Command associatedCommand)
{
    Configuration::Settings defaultSettings;

    m_settings->shortcutsMap[associatedCommand] = defaultSettings.shortcutsMap[associatedCommand];
}

void ConfigManager::resetAllShortcuts()
{
    Configuration::Settings defaultSettings;

    m_settings->shortcutsMap = defaultSettings.shortcutsMap;
}

void ConfigManager::resetSettings()
{
    delete m_settings;
    m_settings = new Configuration::Settings;
}

// SETTERS
// Editor settings
void ConfigManager::setTabSize(unsigned int nbOfSpaces)
{
    m_settings->tabSize = nbOfSpaces;
}

bool ConfigManager::setDefaultProjectsPath(QString defaultPath)
{
    if (defaultPath.isEmpty())
        return false;

    m_settings->defaultProjectsPath = defaultPath;
    saveConfigFile();

    return true;
}

// Assembler settings
void ConfigManager::setRamAsDefaultMemoryTarget(bool ramAsDefault)
{
    m_settings->ramAsDefaultMemoryTarget = ramAsDefault;
}

// Emulator settings
void ConfigManager::setStartEmulatorPaused(bool paused)
{
    m_settings->startEmulatorPaused = paused;
    saveConfigFile();
}

void ConfigManager::setMonitorPlugged(bool plugged)
{
    m_settings->monitorPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setRTCPlugged(bool plugged)
{
    m_settings->rtcPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setKeyboardPlugged(bool plugged)
{
    m_settings->keyboardPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setEepromPlugged(bool plugged)
{
    m_settings->eepromPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setDismissReassemblyWarnings(bool dismiss)
{
    m_settings->dismissReassemblyWarnings = dismiss;
    saveConfigFile();
}

void ConfigManager::setPixelScale(unsigned int scale)
{
    m_settings->pixelScale = scale;
}

// Cpu state viewer settings
void ConfigManager::setOpenCpuStateViewerOnEmulatorPaused(bool enable)
{
    m_settings->openCpuStateViewerOnEmulatorPaused = enable;
}

void ConfigManager::setOpenCpuStateViewerOnEmulatorStopped(bool enable)
{
    m_settings->openCpuStateViewerOnEmulatorStopped = enable;
}

// Binary viewer settings
void ConfigManager::setOpenBinaryViewerOnAssembly(bool enable)
{
    m_settings->openBinaryViewerOnAssembly = enable;
}

void ConfigManager::setOpenBinaryViewerOnEmulatorPaused(bool enable)
{
    m_settings->openBinaryViewerOnEmulatorPaused = enable;
}

void ConfigManager::setOpenBinaryViewerOnEmulatorStopped(bool enable)
{
    m_settings->openBinaryViewerOnEmulatorStopped = enable;
}

// Disassembly viewer settings
void ConfigManager::setOpenDisassemblyViewerOnAssembly(bool enable)
{
    m_settings->openDisassemblyViewerOnAssembly = enable;
}

void ConfigManager::setOpenDisassemblyViewerOnEmulatorPaused(bool enable)
{
    m_settings->openDisassemblyViewerOnEmulatorPaused = enable;
}

void ConfigManager::setOpenDisassemblyViewerOnEmulatorStopped(bool enable)
{
    m_settings->openDisassemblyViewerOnEmulatorStopped = enable;
}

// GETTERS
// Editor settings
unsigned int ConfigManager::getTabSize()
{
    return m_settings->tabSize;
}

QString ConfigManager::getDefaultProjectsPath()
{
    return m_settings->defaultProjectsPath;
}

std::map<Configuration::Command, QKeySequence>& ConfigManager::getShortcutsMap()
{
    return m_settings->shortcutsMap;
}

// Assembler settings
bool ConfigManager::getRamAsDefaultMemoryTarget()
{
    return m_settings->ramAsDefaultMemoryTarget;
}

// Emulator settings
bool ConfigManager::getStartEmulatorPaused()
{
    return m_settings->startEmulatorPaused;
}

bool ConfigManager::getMonitorPlugged()
{
    return m_settings->monitorPlugged;
}

bool ConfigManager::getRTCPlugged()
{
    return m_settings->rtcPlugged;
}

bool ConfigManager::getKeyboardPlugged()
{
    return m_settings->keyboardPlugged;
}

bool ConfigManager::getEepromPlugged()
{
    return m_settings->eepromPlugged;
}

bool ConfigManager::getDismissReassemblyWarnings()
{
    return m_settings->dismissReassemblyWarnings;
}

unsigned int ConfigManager::getPixelScale()
{
    return m_settings->pixelScale;
}

// Cpu state viewer settings
bool ConfigManager::getOpenCpuStateViewerOnEmulatorPaused()
{
    return m_settings->openCpuStateViewerOnEmulatorPaused;
}

bool ConfigManager::getOpenCpuStateViewerOnEmulatorStopped()
{
    return m_settings->openCpuStateViewerOnEmulatorStopped;
}

// Binary viewer settings
bool ConfigManager::getOpenBinaryViewerOnAssembly()
{
    return m_settings->openBinaryViewerOnAssembly;
}

bool ConfigManager::getOpenBinaryViewerOnEmulatorPaused()
{
    return m_settings->openBinaryViewerOnEmulatorPaused;
}

bool ConfigManager::getOpenBinaryViewerOnEmulatorStopped()
{
    return m_settings->openBinaryViewerOnEmulatorStopped;
}

// Disassembly viewer settings
bool ConfigManager::getOpenDisassemblyViewerOnAssembly()
{
    return m_settings->openDisassemblyViewerOnAssembly;
}

bool ConfigManager::getOpenDisassemblyViewerOnEmulatorPaused()
{
    return m_settings->openDisassemblyViewerOnEmulatorPaused;
}

bool ConfigManager::getOpenDisassemblyViewerOnEmulatorStopped()
{
    return m_settings->openDisassemblyViewerOnEmulatorStopped;
}

// PRIVATE
ConfigManager::ConfigManager()
{
    m_settings = new Configuration::Settings;
    m_recentProjects.clear();

    // Configuration file
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

                    if (key == "TAB_SIZE")
                    {
                        bool ok;
                        unsigned int tabSize(value.toUInt(&ok));

                        if (ok)
                            m_settings->tabSize = tabSize;
                    }
                    else if (key == "PIXEL_SCALE")
                    {
                        bool ok;
                        unsigned int scale(value.toUInt(&ok));

                        if (ok)
                            m_settings->pixelScale = scale;
                    }
                    else if (key == "RAM_AS_DEFAULT_MEMORY_TARGET")
                    {
                        m_settings->ramAsDefaultMemoryTarget = (value == "TRUE");
                    }
                    else if (key == "DEFAULT_PROJECT_PATH")
                    {
                        if (!value.isEmpty())
                            m_settings->defaultProjectsPath = value;

                        QString defaultProjectsDirPath(m_settings->defaultProjectsPath);
                        QDir defaultProjectsDir(defaultProjectsDirPath);

                        if (!defaultProjectsDir.exists())
                        {
                            defaultProjectsDir.mkpath(defaultProjectsDirPath);
                        }
                    }
                    else if (key == "START_EMULATOR_PAUSED")
                    {
                        m_settings->monitorPlugged = (value == "TRUE");
                    }
                    else if (key == "MONITOR_PLUGGED")
                    {
                        m_settings->monitorPlugged = (value == "TRUE");
                    }
                    else if (key == "RTC_PLUGGED")
                    {
                        m_settings->rtcPlugged = (value == "TRUE");
                    }
                    else if (key == "KEYBOARD_PLUGGED")
                    {
                        m_settings->keyboardPlugged = (value == "TRUE");
                    }
                    else if (key == "EEPROM_PLUGGED")
                    {
                        m_settings->eepromPlugged = (value == "TRUE");
                    }
                    else if (key == "DISMISS_REASSEMBLY_WARNINGS")
                    {
                        m_settings->dismissReassemblyWarnings = (value == "TRUE");
                    }
                    else if (key == "OPEN_CPU_STATE_VIEWER_EMULATOR_PAUSED")
                    {
                        m_settings->openCpuStateViewerOnEmulatorPaused = (value == "TRUE");
                    }
                    else if (key == "OPEN_CPU_STATE_VIEWER_EMULATOR_STOPPED")
                    {
                        m_settings->openCpuStateViewerOnEmulatorStopped = (value == "TRUE");
                    }
                    else if (key == "OPEN_BIN_VIEWER_ASSEMBLY")
                    {
                        m_settings->openBinaryViewerOnAssembly = (value == "TRUE");
                    }
                    else if (key == "OPEN_BIN_VIEWER_EMULATOR_PAUSED")
                    {
                        m_settings->openBinaryViewerOnEmulatorPaused = (value == "TRUE");
                    }
                    else if (key == "OPEN_BIN_VIEWER_EMULATOR_STOPPPED")
                    {
                        m_settings->openBinaryViewerOnEmulatorStopped = (value == "TRUE");
                    }
                    else if (key == "OPEN_DIS_VIEWER_ASSEMBLY")
                    {
                        m_settings->openDisassemblyViewerOnAssembly = (value == "TRUE");
                    }
                    else if (key == "OPEN_DIS_VIEWER_EMULATOR_PAUSED")
                    {
                        m_settings->openDisassemblyViewerOnEmulatorPaused = (value == "TRUE");
                    }
                    else if (key == "OPEN_DIS_VIEWER_EMULATOR_STOPPPED")
                    {
                        m_settings->openDisassemblyViewerOnEmulatorStopped = (value == "TRUE");
                    }
                    else if (key == "NEW_PROJECT_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::NEW_PROJECT] = QKeySequence::fromString(value);
                    }
                    else if (key == "NEW_FILE_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::NEW_FILE] = QKeySequence::fromString(value);
                    }
                    else if (key == "OPEN_PROJECT_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::OPEN_PROJECT] = QKeySequence::fromString(value);
                    }
                    else if (key == "OPEN_FILE_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::OPEN_FILE] = QKeySequence::fromString(value);
                    }
                    else if (key == "SAVE_CURRENT_FILE_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::SAVE_CURRENT_FILE] = QKeySequence::fromString(value);
                    }
                    else if (key == "SAVE_ALL_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::SAVE_ALL] = QKeySequence::fromString(value);
                    }
                    else if (key == "CLOSE_CURRENT_PROJECT_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::CLOSE_CURRENT_PROJECT] = QKeySequence::fromString(value);
                    }
                    else if (key == "CLOSE_CURRENT_FILE_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::CLOSE_CURRENT_FILE] = QKeySequence::fromString(value);
                    }
                    else if (key == "CLOSE_ALL_FILES_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::CLOSE_ALL_FILES] = QKeySequence::fromString(value);
                    }
                    else if (key == "QUIT_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::QUIT] = QKeySequence::fromString(value);
                    }
                    else if (key == "ASSEMBLE_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::ASSEMBLE] = QKeySequence::fromString(value);
                    }
                    else if (key == "SHOW_BIN_OUTPUT_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::SHOW_BIN_OUTPUT] = QKeySequence::fromString(value);
                    }
                    else if (key == "SHOW_DISASSEMBLY_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::SHOW_DISASSEMBLY] = QKeySequence::fromString(value);
                    }
                    else if (key == "SHOW_CPU_STATE_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::SHOW_CPU_STATE] = QKeySequence::fromString(value);
                    }
                    else if (key == "RUN_EMULATOR_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::RUN_EMULATOR] = QKeySequence::fromString(value);
                    }
                    else if (key == "STEP_EMULATOR_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::STEP_EMULATOR] = QKeySequence::fromString(value);
                    }
                    else if (key == "PAUSE_EMULATOR_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::PAUSE_EMULATOR] = QKeySequence::fromString(value);
                    }
                    else if (key == "STOP_EMULATOR_SHORTCUT")
                    {
                        m_settings->shortcutsMap[Configuration::Command::STOP_EMULATOR] = QKeySequence::fromString(value);
                    }
                }
            }
        }

        configFile.close();
    }

    // Recent projects file
    QString recentProjectsFilePath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/recentProjects.cfg");
    QFile recentProjectsFile(recentProjectsFilePath);

    if (QFile::exists(configFilePath))
    {
        if (recentProjectsFile.open(QIODevice::ReadOnly))
        {
            QTextStream in(&recentProjectsFile);

            int i(0);
            while (!in.atEnd() && i < 10)
            {
                QString line = in.readLine();

                m_recentProjects.push_back(line);

                i++;
            }

            recentProjectsFile.close();
        }
    }
}

bool ConfigManager::saveConfigFile()
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

        out << "TAB_SIZE=" << QString::number(m_settings->tabSize) << "\n";
        out << "DEFAULT_PROJECT_PATH=" << m_settings->defaultProjectsPath << "\n";

        out << "RAM_AS_DEFAULT_MEMORY_TARGET=" << (m_settings->ramAsDefaultMemoryTarget ? "TRUE" : "FALSE") << "\n";

        out << "START_EMULATOR_PAUSED=" << (m_settings->startEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "MONITOR_PLUGGED=" << (m_settings->monitorPlugged ? "TRUE" : "FALSE") << "\n";
        out << "RTC_PLUGGED=" << (m_settings->rtcPlugged ? "TRUE" : "FALSE") << "\n";
        out << "KEYBOARD_PLUGGED=" << (m_settings->keyboardPlugged ? "TRUE" : "FALSE") << "\n";
        out << "EEPROM_PLUGGED=" << (m_settings->eepromPlugged ? "TRUE" : "FALSE") << "\n";
        out << "DISMISS_REASSEMBLY_WARNINGS=" << (m_settings->dismissReassemblyWarnings ? "TRUE" : "FALSE") << "\n";
        out << "PIXEL_SCALE=" << QString::number(m_settings->pixelScale) << "\n";

        out << "OPEN_CPU_STATE_VIEWER_EMULATOR_PAUSED=" << (m_settings->openCpuStateViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_CPU_STATE_VIEWER_EMULATOR_STOPPED=" << (m_settings->openCpuStateViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";

        out << "OPEN_BIN_VIEWER_ASSEMBLY=" << (m_settings->openBinaryViewerOnAssembly ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_EMULATOR_PAUSED=" << (m_settings->openBinaryViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_EMULATOR_STOPPPED=" << (m_settings->openBinaryViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";

        out << "OPEN_DIS_VIEWER_ASSEMBLY=" << (m_settings->openDisassemblyViewerOnAssembly ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_DIS_VIEWER_EMULATOR_PAUSED=" << (m_settings->openDisassemblyViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_DIS_VIEWER_EMULATOR_STOPPPED=" << (m_settings->openDisassemblyViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";

        out << "NEW_PROJECT_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::NEW_PROJECT].toString() << "\n";
        out << "NEW_FILE_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::NEW_FILE].toString() << "\n";
        out << "OPEN_PROJECT_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::OPEN_PROJECT].toString() << "\n";
        out << "OPEN_FILE_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::OPEN_FILE].toString() << "\n";
        out << "SAVE_CURRENT_FILE_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::SAVE_CURRENT_FILE].toString() << "\n";
        out << "SAVE_ALL_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::SAVE_ALL].toString() << "\n";
        out << "CLOSE_CURRENT_PROJECT_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::CLOSE_CURRENT_PROJECT].toString() << "\n";
        out << "CLOSE_CURRENT_FILE_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::CLOSE_CURRENT_FILE].toString() << "\n";
        out << "CLOSE_ALL_FILES_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::CLOSE_ALL_FILES].toString() << "\n";
        out << "QUIT_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::QUIT].toString() << "\n";
        out << "ASSEMBLE_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::ASSEMBLE].toString() << "\n";
        out << "SHOW_BIN_OUTPUT_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::SHOW_BIN_OUTPUT].toString() << "\n";
        out << "SHOW_DISASSEMBLY_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::SHOW_DISASSEMBLY].toString() << "\n";
        out << "SHOW_CPU_STATE_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::SHOW_CPU_STATE].toString() << "\n";
        out << "RUN_EMULATOR_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::RUN_EMULATOR].toString() << "\n";
        out << "STEP_EMULATOR_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::STEP_EMULATOR].toString() << "\n";
        out << "PAUSE_EMULATOR_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::PAUSE_EMULATOR].toString() << "\n";
        out << "STOP_EMULATOR_SHORTCUT=" << m_settings->shortcutsMap[Configuration::Command::STOP_EMULATOR].toString() << "\n";

        configFile.close();
        return true;
    }

    return false;
}

bool ConfigManager::saveRecentProjects()
{
    QString recentProjectsFilePath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/recentProjects.cfg");
    QFile recentProjectsFile(recentProjectsFilePath);

    QString recentProjectsDirectoryPath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation));
    QDir recentProjectsDirectory(recentProjectsDirectoryPath);

    if (!recentProjectsDirectory.exists())
    {
        recentProjectsDirectory.mkpath(recentProjectsDirectoryPath);
    }

    if (recentProjectsFile.open(QIODevice::WriteOnly))
    {
        QTextStream out(&recentProjectsFile);

        for (unsigned int i(0); i < m_recentProjects.size(); i++)
        {
            out << m_recentProjects[i] << "\n";
        }

        recentProjectsFile.close();
        return true;
    }

    return false;
}

void ConfigManager::flushRecentProjects()
{
    m_recentProjects.removeDuplicates();

    for (auto &p : m_recentProjects)
    {
        if (p.isEmpty())
        {
            m_recentProjects.removeOne(p);
        }

        if (!QFileInfo(p).isFile())
        {
            m_recentProjects.removeOne(p);
        }

        if (QFileInfo(p).completeSuffix() != "hcp") // HBC-2 project files
        {
            m_recentProjects.removeOne(p);
        }
    }
}


// ========= SettingsDialog class =========
SettingsDialog::SettingsDialog(ConfigManager *configManager, MainWindow *mainWin) : QDialog(qobject_cast<QWidget*>(mainWin))
{
    m_configManager = configManager;
    setWindowSettings();

    initMenu();

    QHBoxLayout *panelLayout = new QHBoxLayout;
    panelLayout->addWidget(m_menuList);
    panelLayout->addWidget(m_menuWidgets);

    m_resetAllButton = new QPushButton(tr("Set default settings"));
    connect(m_resetAllButton, SIGNAL(clicked()), this, SLOT(resetAllAction()));

    m_closeButton = new QPushButton(tr("Close"), this);
    connect(m_closeButton, SIGNAL(clicked()), this, SLOT(accept()));
    connect(this, SIGNAL(accepted()), mainWin, SLOT(onSettingsChanged()));

    QHBoxLayout *buttonsLayout = new QHBoxLayout;
    buttonsLayout->addWidget(m_resetAllButton);
    buttonsLayout->addWidget(m_closeButton);

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addLayout(panelLayout);
    mainLayout->addLayout(buttonsLayout);

    setLayout(mainLayout);
}

// PRIVATE SLOTS
void SettingsDialog::menuSelected(int tabIndex)
{
    m_menuWidgets->setCurrentWidget(m_menuWidgetsList[tabIndex]);
}

void SettingsDialog::tabSizeChanged(int nbOfSpaces)
{
    m_configManager->setTabSize(nbOfSpaces);
}

void SettingsDialog::defaultProjectsPathChanged(QString newPath)
{
    if (newPath.isEmpty())
    {
        QMessageBox::warning(this, tr("Invalid projects directory"), tr("The projects directory cannot be empty"));
    }
    else
    {
        m_configManager->setDefaultProjectsPath(newPath);
    }
}

void SettingsDialog::shortcutSelected()
{
    QString selectedCommandStr(m_keyboardShortcutsTableWidget->selectedItems()[0]->text());

    for (unsigned int i(0); i < Configuration::SHORTCUTS_COUNT; i++)
    {
        if (Configuration::commandStrArr[i] == selectedCommandStr)
        {
            m_keySequenceEdit->setKeySequence(m_configManager->getShortcutsMap()[(Configuration::Command)i]);
            break;
        }
    }

    m_keySequenceEdit->setEnabled(true);
    m_keyboardShortcutsSaveButton->setEnabled(true);
    m_keyboardShortcutsResetButton->setEnabled(true);
}

void SettingsDialog::saveButtonClicked()
{
    QString selectedCommandStr(m_keyboardShortcutsTableWidget->selectedItems()[0]->text());

    for (unsigned int i(0); i < Configuration::SHORTCUTS_COUNT; i++)
    {
        if (Configuration::commandStrArr[i] == selectedCommandStr)
        {
            m_keyboardShortcutsTableWidget->selectedItems()[1]->setText(m_keySequenceEdit->keySequence().toString());
            m_configManager->getShortcutsMap()[(Configuration::Command)i] = m_keySequenceEdit->keySequence();
            break;
        }
    }
}

void SettingsDialog::resetSelectedShortcut()
{
    QString selectedCommandStr(m_keyboardShortcutsTableWidget->selectedItems()[0]->text());

    for (unsigned int i(0); i < Configuration::SHORTCUTS_COUNT; i++)
    {
        if (Configuration::commandStrArr[i] == selectedCommandStr)
        {
            m_configManager->resetShortcut((Configuration::Command)i);
            break;
        }
    }

    updateWidgets();
}

void SettingsDialog::resetAllShortcuts()
{
    m_configManager->resetAllShortcuts();
    updateWidgets();
}

void SettingsDialog::browseProjectsPathClicked()
{
    QString newDefaultProjectsPath = QFileDialog::getExistingDirectory(this, tr("Select new projects directory"), m_configManager->getDefaultProjectsPath(), QFileDialog::ShowDirsOnly);

    m_defaultProjectsPathLineEdit->setText(newDefaultProjectsPath);
}

void SettingsDialog::ramAsDefaultMemoryTargetChanged()
{
    m_configManager->setRamAsDefaultMemoryTarget(m_ramAsDefaultMemoryTargetCheckBox->isChecked());
}

void SettingsDialog::startPausedChanged()
{
    m_configManager->setStartEmulatorPaused(m_startPausedCheckBox->isChecked());
}

void SettingsDialog::plugMonitorChanged()
{
    m_configManager->setMonitorPlugged(m_plugMonitorCheckBox->isChecked());
}

void SettingsDialog::plugRTCChanged()
{
    m_configManager->setRTCPlugged(m_plugRTCCheckBox->isChecked());
}

void SettingsDialog::plugKeyboardChanged()
{
    m_configManager->setKeyboardPlugged(m_plugKeyboardCheckBox->isChecked());
}

void SettingsDialog::plugEepromChanged()
{
    m_configManager->setEepromPlugged(m_plugEepromCheckBox->isChecked());
}

void SettingsDialog::dismissReassemblyWarningsChanged()
{
    m_configManager->setDismissReassemblyWarnings(m_dismissReassemblyWarningsCheckBox->isChecked());
}

void SettingsDialog::pixelScaleChanged(int scale)
{
    m_configManager->setPixelScale(scale);
}

void SettingsDialog::openCpuStateViewerOnEmulatorPausedChanged()
{
    m_configManager->setOpenCpuStateViewerOnEmulatorPaused(m_openCpuStateViewerOnEmulatorPausedCheckBox->isChecked());
}

void SettingsDialog::openCpuStateViewerOnEmulatorStoppedChanged()
{
    m_configManager->setOpenCpuStateViewerOnEmulatorStopped(m_openCpuStateViewerOnEmulatorStoppedCheckBox->isChecked());
}

void SettingsDialog::openBinaryViewerOnAssemblyChanged()
{
    m_configManager->setOpenBinaryViewerOnAssembly(m_openBinaryViewerOnAssemblyCheckBox->isChecked());
}

void SettingsDialog::openBinaryViewerOnEmulatorPausedChanged()
{
    m_configManager->setOpenBinaryViewerOnEmulatorPaused(m_openBinaryViewerOnEmulatorPausedCheckBox->isChecked());
}

void SettingsDialog::openBinaryViewerOnEmulatorStoppedChanged()
{
    m_configManager->setOpenBinaryViewerOnEmulatorStopped(m_openBinaryViewerOnEmulatorStoppedCheckBox->isChecked());
}

void SettingsDialog::openDisassemblyViewerOnAssemblyChanged()
{
    m_configManager->setOpenDisassemblyViewerOnAssembly(m_openDisassemblyViewerOnAssemblyCheckBox->isChecked());
}

void SettingsDialog::openDisassemblyViewerOnEmulatorPausedChanged()
{
    m_configManager->setOpenDisassemblyViewerOnEmulatorPaused(m_openDisassemblyViewerOnEmulatorPausedCheckBox->isChecked());
}

void SettingsDialog::openDisassemblyViewerOnEmulatorStoppedChanged()
{
    m_configManager->setOpenDisassemblyViewerOnEmulatorStopped(m_openDisassemblyViewerOnEmulatorStoppedCheckBox->isChecked());
}

void SettingsDialog::resetAllAction()
{
    m_configManager->resetSettings();
    updateWidgets();
}

// PRIVATE
void SettingsDialog::initMenu()
{
    // Init layouts
    initEditorSettingsLayout();
    initAssemblerSettingsLayout();
    initEmulatorSettingsLayout();
    initCpuStateViewerSettingsLayout();
    initBinaryViewerSettingsLayout();
    initDisassemblyViewerSettingsLayout();

    // Set the window main layout
    m_menuWidgets = new QStackedWidget(this);
    m_menuList = new QListWidget(this);
    m_menuList->setFixedWidth(MENU_LIST_WIDTH);

    addMenu(m_editorSettingsItem, m_editorSettingsPageWidget);
    addMenu(m_assemblerSettingsItem, m_assemblerSettingsPageWidget);
    addMenu(m_emulatorSettingsItem, m_emulatorSettingsPageWidget);
    addMenu(m_cpuStateViewerSettingsItem, m_cpuStateViewerSettingsPageWidget);
    addMenu(m_binaryViewerSettingsItem, m_binaryViewerSettingsPageWidget);
    addMenu(m_disassemblyViewerSettingsItem, m_disassemblyViewerSettingsPageWidget);

    m_menuList->setCurrentRow(0);
    m_menuWidgets->setCurrentIndex(0);

    updateWidgets();

    connect(m_menuList, SIGNAL(currentRowChanged(int)), this, SLOT(menuSelected(int)));
}

void SettingsDialog::setWindowSettings()
{
    setFixedSize(DIALOG_WIDTH, DIALOG_HEIGHT);
    setWindowTitle(tr("Settings"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));
}

void SettingsDialog::initEditorSettingsLayout()
{
    // Layout init
    m_editorSettingsPageWidget = new QWidget(this);
    m_editorSettingsPageLayout = new QVBoxLayout;

    m_editorSettingsItem = new QListWidgetItem(tr("Editor"));
    m_editorSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // Tabs
    m_editorSettingsTabWidget = new QTabWidget(this);
    m_editorSettingsGeneralTabLayout = new QVBoxLayout;
    m_editorSettingsGeneralTabWidget = new QWidget(qobject_cast<QWidget*>(m_editorSettingsPageLayout));

    m_editorSettingsKeyboardTabLayout = new QVBoxLayout;
    m_editorSettingsKeyboardTabWidget = new QWidget(qobject_cast<QWidget*>(m_editorSettingsPageLayout));

    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Editor"), qobject_cast<QWidget*>(m_editorSettingsPageLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    QLabel *defaultProjectsPathLabel = new QLabel(tr("Projects directory"), qobject_cast<QWidget*>(m_editorSettingsGeneralTabLayout));
    m_browseProjectsPath = new QPushButton(tr("Browse"), qobject_cast<QWidget*>(m_editorSettingsGeneralTabLayout));
    m_defaultProjectsPathLineEdit = new QLineEdit(qobject_cast<QWidget*>(m_editorSettingsGeneralTabLayout));
    m_defaultProjectsPathLineEdit->setMinimumWidth(250);
    QHBoxLayout *defaultProjectsPathLayout = new QHBoxLayout;
    defaultProjectsPathLayout->addWidget(m_defaultProjectsPathLineEdit);
    defaultProjectsPathLayout->addWidget(m_browseProjectsPath);

    QLabel *tabSizeLabel = new QLabel(tr("Tab size"), qobject_cast<QWidget*>(m_editorSettingsGeneralTabLayout));
    m_tabSizeSpinBox = new QSpinBox(qobject_cast<QWidget*>(m_editorSettingsGeneralTabLayout));
    m_tabSizeSpinBox->setSuffix(" spaces");
    m_tabSizeSpinBox->setRange(1, 16);
    QHBoxLayout *tabSizeLayout = new QHBoxLayout;
    tabSizeLayout->addWidget(tabSizeLabel);
    tabSizeLayout->addWidget(m_tabSizeSpinBox);

    QStringList header;
    m_keyboardShortcutsTableWidget = new QTableWidget(Configuration::SHORTCUTS_COUNT, 2, this);
    header << tr("Command") << tr("Shortcut");
    m_keyboardShortcutsTableWidget->setHorizontalHeaderLabels(header);
    m_keyboardShortcutsTableWidget->setEditTriggers(QAbstractItemView::NoEditTriggers);
    m_keyboardShortcutsTableWidget->setFocusPolicy(Qt::NoFocus);
    m_keyboardShortcutsTableWidget->setSelectionBehavior(QAbstractItemView::SelectRows);
    m_keyboardShortcutsTableWidget->setSelectionMode(QAbstractItemView::SingleSelection);
    m_keyboardShortcutsTableWidget->verticalHeader()->setVisible(false);
    m_keyboardShortcutsTableWidget->horizontalHeader()->setSectionResizeMode(QHeaderView::Fixed);
    m_keyboardShortcutsTableWidget->setColumnWidth(0, 400);
    m_keyboardShortcutsTableWidget->setColumnWidth(1, 156);
    for (unsigned int i(0); i < Configuration::SHORTCUTS_COUNT; i++)
    {
        m_keyboardShortcutsTableWidget->setItem(i, 0, new QTableWidgetItem);
        m_keyboardShortcutsTableWidget->setItem(i, 1, new QTableWidgetItem);
    }

    m_keyboardShortcutsResetButton = new QPushButton(tr("Reset"), this);
    m_keyboardShortcutsResetButton->setEnabled(false);
    QPushButton *keyboardShortcutsResetAllButton = new QPushButton(tr("Reset all"), this);
    QHBoxLayout *keyboardShortcutsResetLayout = new QHBoxLayout;
    keyboardShortcutsResetLayout->addWidget(m_keyboardShortcutsResetButton);
    keyboardShortcutsResetLayout->addWidget(keyboardShortcutsResetAllButton);

    m_keySequenceEdit = new CustomKeySequenceEdit(this);
    m_keySequenceEdit->setEnabled(false);
    m_keyboardShortcutsSaveButton = new QPushButton(tr("Save shortcut"), this);
    m_keyboardShortcutsSaveButton->setEnabled(false);
    QHBoxLayout *keyboardShortcutsSaveLayout = new QHBoxLayout;
    keyboardShortcutsSaveLayout->addWidget(m_keySequenceEdit);
    keyboardShortcutsSaveLayout->addWidget(m_keyboardShortcutsSaveButton);
    // ------------------


    // Final layout configuration (per tab)
    m_editorSettingsGeneralTabLayout->addWidget(defaultProjectsPathLabel);
    m_editorSettingsGeneralTabLayout->addLayout(defaultProjectsPathLayout);
    m_editorSettingsGeneralTabLayout->addLayout(tabSizeLayout);
    m_editorSettingsGeneralTabLayout->addStretch();
    m_editorSettingsGeneralTabWidget->setLayout(m_editorSettingsGeneralTabLayout);
    m_editorSettingsTabWidget->addTab(m_editorSettingsGeneralTabWidget, tr("General"));

    m_editorSettingsKeyboardTabLayout->addWidget(m_keyboardShortcutsTableWidget);
    m_editorSettingsKeyboardTabLayout->addLayout(keyboardShortcutsResetLayout);
    m_editorSettingsKeyboardTabLayout->addLayout(keyboardShortcutsSaveLayout);
    m_editorSettingsKeyboardTabWidget->setLayout(m_editorSettingsKeyboardTabLayout);
    m_editorSettingsTabWidget->addTab(m_editorSettingsKeyboardTabWidget, tr("Keyboard"));

    m_editorSettingsPageLayout->addWidget(mainLabel);
    m_editorSettingsPageLayout->addWidget(m_editorSettingsTabWidget);
    m_editorSettingsPageWidget->setLayout(m_editorSettingsPageLayout);


    // Connections
    connect(m_tabSizeSpinBox, SIGNAL(valueChanged(int)), this, SLOT(tabSizeChanged(int)));
    connect(m_browseProjectsPath, SIGNAL(clicked()), this, SLOT(browseProjectsPathClicked()));
    connect(m_defaultProjectsPathLineEdit, SIGNAL(textChanged(QString)), this, SLOT(defaultProjectsPathChanged(QString)));
    connect(m_keyboardShortcutsTableWidget, SIGNAL(itemSelectionChanged()), this, SLOT(shortcutSelected()));
    connect(m_keyboardShortcutsSaveButton, SIGNAL(clicked()), this, SLOT(saveButtonClicked()));
    connect(m_keyboardShortcutsResetButton, SIGNAL(clicked()), this, SLOT(resetSelectedShortcut()));
    connect(keyboardShortcutsResetAllButton, SIGNAL(clicked()), this, SLOT(resetAllShortcuts()));
}

void SettingsDialog::initAssemblerSettingsLayout()
{
    // Layout init
    m_assemblerSettingsPageWidget = new QWidget(this);
    m_assemblerSettingsPageLayout = new QVBoxLayout;

    m_assemblerSettingsItem = new QListWidgetItem(tr("Assembler"));
    m_assemblerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // Tabs
    m_assemblerSettingsTabWidget = new QTabWidget(this);
    m_assemblerSettingsGeneralTabLayout = new QVBoxLayout;
    m_assemblerSettingsGeneralTabWidget = new QWidget(qobject_cast<QWidget*>(m_assemblerSettingsGeneralTabLayout));


    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Assembler"), qobject_cast<QWidget*>(m_assemblerSettingsPageLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_ramAsDefaultMemoryTargetCheckBox = new QCheckBox(tr("RAM as default memory target"), qobject_cast<QWidget*>(m_assemblerSettingsGeneralTabLayout));
    // ------------------


    // Final layout configuration
    m_assemblerSettingsGeneralTabLayout->addWidget(m_ramAsDefaultMemoryTargetCheckBox);
    m_assemblerSettingsGeneralTabLayout->addStretch();
    m_assemblerSettingsGeneralTabWidget->setLayout(m_assemblerSettingsGeneralTabLayout);
    m_assemblerSettingsTabWidget->addTab(m_assemblerSettingsGeneralTabWidget, tr("General"));

    m_assemblerSettingsPageLayout->addWidget(mainLabel);
    m_assemblerSettingsPageLayout->addWidget(m_assemblerSettingsTabWidget);
    m_assemblerSettingsPageWidget->setLayout(m_assemblerSettingsPageLayout);


    // Connections
    connect(m_ramAsDefaultMemoryTargetCheckBox, SIGNAL(stateChanged(int)), this, SLOT(ramAsDefaultMemoryTargetChanged()));
}

void SettingsDialog::initEmulatorSettingsLayout()
{
    // Layout init
    m_emulatorSettingsPageWidget = new QWidget(this);
    m_emulatorSettingsPageLayout = new QVBoxLayout;

    m_emulatorSettingsItem = new QListWidgetItem(tr("Emulator"));
    m_emulatorSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // Tabs
    m_emulatorSettingsTabWidget = new QTabWidget(this);
    m_emulatorSettingsGeneralTabLayout = new QVBoxLayout;
    m_emulatorSettingsGeneralTabWidget = new QWidget(qobject_cast<QWidget*>(m_emulatorSettingsGeneralTabLayout));

    m_emulatorSettingsMonitorTabLayout = new QVBoxLayout;
    m_emulatorSettingsMonitorTabWidget = new QWidget(qobject_cast<QWidget*>(m_emulatorSettingsMonitorTabLayout));

    m_emulatorSettingsRtcTabLayout = new QVBoxLayout;
    m_emulatorSettingsRtcTabWidget = new QWidget(qobject_cast<QWidget*>(m_emulatorSettingsRtcTabLayout));

    m_emulatorSettingsKeyboardTabLayout = new QVBoxLayout;
    m_emulatorSettingsKeyboardTabWidget = new QWidget(qobject_cast<QWidget*>(m_emulatorSettingsKeyboardTabLayout));

    m_emulatorSettingsEepromTabLayout = new QVBoxLayout;
    m_emulatorSettingsEepromTabWidget = new QWidget(qobject_cast<QWidget*>(m_emulatorSettingsEepromTabLayout));


    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Emulator"), qobject_cast<QWidget*>(m_emulatorSettingsPageLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_startPausedCheckBox = new QCheckBox(tr("Start paused"), qobject_cast<QWidget*>(m_emulatorSettingsGeneralTabLayout));
    m_dismissReassemblyWarningsCheckBox = new QCheckBox(tr("Dismiss warnings when running an unassembled project"), qobject_cast<QWidget*>(m_emulatorSettingsGeneralTabLayout));
    m_plugMonitorCheckBox = new QCheckBox(tr("Monitor plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsMonitorTabLayout));

    QLabel *pixelScaleLabel = new QLabel(tr("Pixel scale"), qobject_cast<QWidget*>(m_emulatorSettingsMonitorTabLayout));
    m_pixelScaleSpinBox = new QSpinBox(qobject_cast<QWidget*>(m_editorSettingsGeneralTabLayout));
    m_pixelScaleSpinBox->setRange(1, 8);
    QHBoxLayout *pixelScaleLayout = new QHBoxLayout;
    pixelScaleLayout->addWidget(pixelScaleLabel);
    pixelScaleLayout->addWidget(m_pixelScaleSpinBox);

    m_plugRTCCheckBox = new QCheckBox(tr("Real Time Clock plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsRtcTabLayout));
    m_plugKeyboardCheckBox = new QCheckBox(tr("Keyboard plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsKeyboardTabLayout));
    m_plugEepromCheckBox = new QCheckBox(tr("EEPROM plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsEepromTabLayout));
    // ------------------


    // Final layout configuration
    m_emulatorSettingsGeneralTabLayout->addWidget(m_startPausedCheckBox);
    m_emulatorSettingsGeneralTabLayout->addWidget(m_dismissReassemblyWarningsCheckBox);
    m_emulatorSettingsGeneralTabLayout->addStretch();
    m_emulatorSettingsGeneralTabWidget->setLayout(m_emulatorSettingsGeneralTabLayout);

    m_emulatorSettingsMonitorTabLayout->addWidget(m_plugMonitorCheckBox);
    m_emulatorSettingsMonitorTabLayout->addLayout(pixelScaleLayout);
    m_emulatorSettingsMonitorTabLayout->addStretch();
    m_emulatorSettingsMonitorTabWidget->setLayout(m_emulatorSettingsMonitorTabLayout);

    m_emulatorSettingsRtcTabLayout->addWidget(m_plugRTCCheckBox);
    m_emulatorSettingsRtcTabLayout->addStretch();
    m_emulatorSettingsRtcTabWidget->setLayout(m_emulatorSettingsRtcTabLayout);

    m_emulatorSettingsKeyboardTabLayout->addWidget(m_plugKeyboardCheckBox);
    m_emulatorSettingsKeyboardTabLayout->addStretch();
    m_emulatorSettingsKeyboardTabWidget->setLayout(m_emulatorSettingsKeyboardTabLayout);

    m_emulatorSettingsEepromTabLayout->addWidget(m_plugEepromCheckBox);
    m_emulatorSettingsEepromTabLayout->addStretch();
    m_emulatorSettingsEepromTabWidget->setLayout(m_emulatorSettingsEepromTabLayout);

    m_emulatorSettingsTabWidget->addTab(m_emulatorSettingsGeneralTabWidget, tr("General"));
    m_emulatorSettingsTabWidget->addTab(m_emulatorSettingsMonitorTabWidget, tr("Monitor"));
    m_emulatorSettingsTabWidget->addTab(m_emulatorSettingsRtcTabWidget, tr("RTC"));
    m_emulatorSettingsTabWidget->addTab(m_emulatorSettingsKeyboardTabWidget, tr("Keyboard"));
    m_emulatorSettingsTabWidget->addTab(m_emulatorSettingsEepromTabWidget, tr("EEPROM"));

    m_emulatorSettingsPageLayout->addWidget(mainLabel);
    m_emulatorSettingsPageLayout->addWidget(m_emulatorSettingsTabWidget);
    m_emulatorSettingsPageWidget->setLayout(m_emulatorSettingsPageLayout);


    // Connections
    connect(m_startPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(startPausedChanged()));
    connect(m_plugMonitorCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugMonitorChanged()));
    connect(m_pixelScaleSpinBox, SIGNAL(valueChanged(int)), this, SLOT(pixelScaleChanged(int)));
    connect(m_plugRTCCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugRTCChanged()));
    connect(m_plugKeyboardCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugKeyboardChanged()));
    connect(m_plugEepromCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugEepromChanged()));
    connect(m_dismissReassemblyWarningsCheckBox, SIGNAL(stateChanged(int)), this, SLOT(dismissReassemblyWarningsChanged()));
}

void SettingsDialog::initCpuStateViewerSettingsLayout()
{
    // Layout init
    m_cpuStateViewerSettingsPageWidget = new QWidget(this);
    m_cpuStateViewerSettingsPageLayout = new QVBoxLayout;

    m_cpuStateViewerSettingsItem = new QListWidgetItem(tr("CPU State Viewer"));
    m_cpuStateViewerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // Tabs
    m_cpuStateViewerSettingsTabWidget = new QTabWidget(this);
    m_cpuStateViewerSettingsGeneralTabLayout = new QVBoxLayout;
    m_cpuStateViewerSettingsGeneralTabWidget = new QWidget(qobject_cast<QWidget*>(m_cpuStateViewerSettingsGeneralTabLayout));


    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("CPU State Viewer"), qobject_cast<QWidget*>(m_cpuStateViewerSettingsPageLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_openCpuStateViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), qobject_cast<QWidget*>(m_cpuStateViewerSettingsGeneralTabLayout));
    m_openCpuStateViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), qobject_cast<QWidget*>(m_cpuStateViewerSettingsGeneralTabLayout));
    // ------------------


    // Final layout configuration
    m_cpuStateViewerSettingsGeneralTabLayout->addWidget(m_openCpuStateViewerOnEmulatorPausedCheckBox);
    m_cpuStateViewerSettingsGeneralTabLayout->addWidget(m_openCpuStateViewerOnEmulatorStoppedCheckBox);
    m_cpuStateViewerSettingsGeneralTabLayout->addStretch();
    m_cpuStateViewerSettingsGeneralTabWidget->setLayout(m_cpuStateViewerSettingsGeneralTabLayout);
    m_cpuStateViewerSettingsTabWidget->addTab(m_cpuStateViewerSettingsGeneralTabWidget, tr("General"));

    m_cpuStateViewerSettingsPageLayout->addWidget(mainLabel);
    m_cpuStateViewerSettingsPageLayout->addWidget(m_cpuStateViewerSettingsTabWidget);
    m_cpuStateViewerSettingsPageWidget->setLayout(m_cpuStateViewerSettingsPageLayout);


    // Connections
    connect(m_openCpuStateViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openCpuStateViewerOnEmulatorPausedChanged()));
    connect(m_openCpuStateViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openCpuStateViewerOnEmulatorStoppedChanged()));
}

void SettingsDialog::initBinaryViewerSettingsLayout()
{
    // Layout init
    m_binaryViewerSettingsPageWidget = new QWidget(this);
    m_binaryViewerSettingsPageLayout = new QVBoxLayout;

    m_binaryViewerSettingsItem = new QListWidgetItem(tr("Binary Viewer"));
    m_binaryViewerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // Tabs
    m_binaryViewerSettingsTabWidget = new QTabWidget(this);
    m_binaryViewerSettingsGeneralTabLayout = new QVBoxLayout;
    m_binaryViewerSettingsGeneralTabWidget = new QWidget(qobject_cast<QWidget*>(m_binaryViewerSettingsGeneralTabLayout));


    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Binary Viewer"), qobject_cast<QWidget*>(m_binaryViewerSettingsPageLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_openBinaryViewerOnAssemblyCheckBox = new QCheckBox(tr("Open viewer after each assembly"), qobject_cast<QWidget*>(m_binaryViewerSettingsGeneralTabLayout));
    m_openBinaryViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), qobject_cast<QWidget*>(m_binaryViewerSettingsGeneralTabLayout));
    m_openBinaryViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), qobject_cast<QWidget*>(m_binaryViewerSettingsGeneralTabLayout));
    // ------------------


    // Final layout configuration
    m_binaryViewerSettingsGeneralTabLayout->addWidget(m_openBinaryViewerOnAssemblyCheckBox);
    m_binaryViewerSettingsGeneralTabLayout->addWidget(m_openBinaryViewerOnEmulatorPausedCheckBox);
    m_binaryViewerSettingsGeneralTabLayout->addWidget(m_openBinaryViewerOnEmulatorStoppedCheckBox);
    m_binaryViewerSettingsGeneralTabLayout->addStretch();
    m_binaryViewerSettingsGeneralTabWidget->setLayout(m_binaryViewerSettingsGeneralTabLayout);
    m_binaryViewerSettingsTabWidget->addTab(m_binaryViewerSettingsGeneralTabWidget, tr("General"));

    m_binaryViewerSettingsPageLayout->addWidget(mainLabel);
    m_binaryViewerSettingsPageLayout->addWidget(m_binaryViewerSettingsTabWidget);
    m_binaryViewerSettingsPageWidget->setLayout(m_binaryViewerSettingsPageLayout);


    // Connections
    connect(m_openBinaryViewerOnAssemblyCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnAssemblyChanged()));
    connect(m_openBinaryViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnEmulatorPausedChanged()));
    connect(m_openBinaryViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnEmulatorStoppedChanged()));
}

void SettingsDialog::initDisassemblyViewerSettingsLayout()
{
    // Layout init
    m_disassemblyViewerSettingsPageWidget = new QWidget(this);
    m_disassemblyViewerSettingsPageLayout = new QVBoxLayout;

    m_disassemblyViewerSettingsItem = new QListWidgetItem(tr("Disassembly Viewer"));
    m_disassemblyViewerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // Tabs
    m_disassemblyViewerSettingsTabWidget = new QTabWidget(this);
    m_disassemblyViewerSettingsGeneralTabLayout = new QVBoxLayout;
    m_disassemblyViewerSettingsGeneralTabWidget = new QWidget(qobject_cast<QWidget*>(m_disassemblyViewerSettingsGeneralTabLayout));


    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Disassembly Viewer"), qobject_cast<QWidget*>(m_disassemblyViewerSettingsPageLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_openDisassemblyViewerOnAssemblyCheckBox = new QCheckBox(tr("Open viewer after each assembly"), qobject_cast<QWidget*>(m_disassemblyViewerSettingsGeneralTabLayout));
    m_openDisassemblyViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), qobject_cast<QWidget*>(m_disassemblyViewerSettingsGeneralTabLayout));
    m_openDisassemblyViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), qobject_cast<QWidget*>(m_disassemblyViewerSettingsGeneralTabLayout));
    // ------------------


    // Final layout configuration
    m_disassemblyViewerSettingsGeneralTabLayout->addWidget(m_openDisassemblyViewerOnAssemblyCheckBox);
    m_disassemblyViewerSettingsGeneralTabLayout->addWidget(m_openDisassemblyViewerOnEmulatorPausedCheckBox);
    m_disassemblyViewerSettingsGeneralTabLayout->addWidget(m_openDisassemblyViewerOnEmulatorStoppedCheckBox);
    m_disassemblyViewerSettingsGeneralTabLayout->addStretch();
    m_disassemblyViewerSettingsGeneralTabWidget->setLayout(m_disassemblyViewerSettingsGeneralTabLayout);
    m_disassemblyViewerSettingsTabWidget->addTab(m_disassemblyViewerSettingsGeneralTabWidget, tr("General"));

    m_disassemblyViewerSettingsPageLayout->addWidget(mainLabel);
    m_disassemblyViewerSettingsPageLayout->addWidget(m_disassemblyViewerSettingsTabWidget);
    m_disassemblyViewerSettingsPageWidget->setLayout(m_disassemblyViewerSettingsPageLayout);


    // Connections
    connect(m_openDisassemblyViewerOnAssemblyCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openDisassemblyViewerOnAssemblyChanged()));
    connect(m_openDisassemblyViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openDisassemblyViewerOnEmulatorPausedChanged()));
    connect(m_openDisassemblyViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openDisassemblyViewerOnEmulatorStoppedChanged()));
}

void SettingsDialog::addMenu(QListWidgetItem *newMenuItem, QWidget *newMenuWidget)
{
    m_menuList->addItem(newMenuItem);
    m_menuWidgetsList.push_back(newMenuWidget);
    m_menuWidgets->addWidget(newMenuWidget);
}

void SettingsDialog::updateWidgets()
{
    // Editor settings
    m_defaultProjectsPathLineEdit->setText(m_configManager->getDefaultProjectsPath());
    m_tabSizeSpinBox->setValue(m_configManager->getTabSize());

    for (unsigned int i(0); i < Configuration::SHORTCUTS_COUNT; i++)
    {
        m_keyboardShortcutsTableWidget->item(i, 0)->setText(Configuration::commandStrArr[i]);
        m_keyboardShortcutsTableWidget->item(i, 1)->setText(m_configManager->getShortcutsMap()[(Configuration::Command)i].toString());
    }

    // Assembler settings
    m_ramAsDefaultMemoryTargetCheckBox->setChecked(m_configManager->getRamAsDefaultMemoryTarget());

    // Emulator settings
    m_startPausedCheckBox->setChecked(m_configManager->getStartEmulatorPaused());
    m_dismissReassemblyWarningsCheckBox->setChecked(m_configManager->getDismissReassemblyWarnings());
    m_plugMonitorCheckBox->setChecked(m_configManager->getMonitorPlugged());
    m_pixelScaleSpinBox->setValue(m_configManager->getPixelScale());
    m_plugRTCCheckBox->setChecked(m_configManager->getRTCPlugged());
    m_plugKeyboardCheckBox->setChecked(m_configManager->getKeyboardPlugged());
    m_plugEepromCheckBox->setChecked(m_configManager->getEepromPlugged());

    // CPU State Viewer settings
    m_openCpuStateViewerOnEmulatorPausedCheckBox->setChecked(m_configManager->getOpenCpuStateViewerOnEmulatorPaused());
    m_openCpuStateViewerOnEmulatorStoppedCheckBox->setChecked(m_configManager->getOpenCpuStateViewerOnEmulatorStopped());

    // Binary viewer settings
    m_openBinaryViewerOnAssemblyCheckBox->setChecked(m_configManager->getOpenBinaryViewerOnAssembly());
    m_openBinaryViewerOnEmulatorPausedCheckBox->setChecked(m_configManager->getOpenBinaryViewerOnEmulatorPaused());
    m_openBinaryViewerOnEmulatorStoppedCheckBox->setChecked(m_configManager->getOpenBinaryViewerOnEmulatorStopped());

    // Disassembly viewer settings
    m_openDisassemblyViewerOnAssemblyCheckBox->setChecked(m_configManager->getOpenDisassemblyViewerOnAssembly());
    m_openDisassemblyViewerOnEmulatorPausedCheckBox->setChecked(m_configManager->getOpenDisassemblyViewerOnEmulatorPaused());
    m_openDisassemblyViewerOnEmulatorStoppedCheckBox->setChecked(m_configManager->getOpenDisassemblyViewerOnEmulatorStopped());
}
