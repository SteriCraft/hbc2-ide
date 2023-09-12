#include "config.h"

#include <QFile>
#include <QDir>
#include <QTextStream>
#include <QDebug>


// ConfigManager class
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
    return m_settings.recentProjects;
}

bool ConfigManager::addRecentProject(QString path)
{
    m_settings.recentProjects.push_front(path);

    flushRecentProjects();

    return true;
}

void ConfigManager::clearRecentProjects()
{
    m_settings.recentProjects.clear();
}

// SETTERS
// Editor settings
void ConfigManager::setTabSize(unsigned int nbOfSpaces)
{
    m_settings.tabSize = nbOfSpaces;
}

// Assembler settings
void ConfigManager::setRamAsDefaultMemoryTarget(bool ramAsDefault)
{
    m_settings.ramAsDefaultMemoryTarget = ramAsDefault;
}

// Cpu state viewer settings
void ConfigManager::setOpenCpuStateViewerOnEmulatorPaused(bool enable)
{
    m_settings.openCpuStateViewerOnEmulatorPaused = enable;
}

void ConfigManager::setOpenCpuStateViewerOnEmulatorStopped(bool enable)
{
    m_settings.openCpuStateViewerOnEmulatorStopped = enable;
}

// Binary viewer settings
void ConfigManager::setOpenBinaryViewerOnAssembly(bool enable)
{
    m_settings.openBinaryViewerOnAssembly = enable;
}

void ConfigManager::setOpenBinaryViewerOnEmulatorPaused(bool enable)
{
    m_settings.openBinaryViewerOnEmulatorPaused = enable;
}
void ConfigManager::setOpenBinaryViewerOnEmulatorStopped(bool enable)
{
    m_settings.openBinaryViewerOnEmulatorStopped = enable;
}

// Emulator settings
void ConfigManager::setStartEmulatorPaused(bool paused)
{
    m_settings.startEmulatorPaused = paused;
    saveConfigFile();
}

void ConfigManager::setMonitorPlugged(bool plugged)
{
    m_settings.monitorPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setRTCPlugged(bool plugged)
{
    m_settings.rtcPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setKeyboardPlugged(bool plugged)
{
    m_settings.keyboardPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setEepromPlugged(bool plugged)
{
    m_settings.eepromPlugged = plugged;
    saveConfigFile();
}

void ConfigManager::setDismissReassemblyWarnings(bool dismiss)
{
    m_settings.dismissReassemblyWarnings = dismiss;
    saveConfigFile();
}

// IDE settings
bool ConfigManager::setDefaultProjectsPath(QString defaultPath)
{
    if (defaultPath.isEmpty())
        return false;

    m_settings.defaultProjectsPath = defaultPath;
    saveConfigFile();

    return true;
}

// GETTERS
// Editor settings
unsigned int ConfigManager::getTabSize()
{
    return m_settings.tabSize;
}

// Assembler settings
bool ConfigManager::getRamAsDefaultMemoryTarget()
{
    return m_settings.ramAsDefaultMemoryTarget;
}

// Cpu state viewer settings
bool ConfigManager::getOpenCpuStateViewerOnEmulatorPaused()
{
    return m_settings.openCpuStateViewerOnEmulatorPaused;
}

bool ConfigManager::getOpenCpuStateViewerOnEmulatorStopped()
{
    return m_settings.openCpuStateViewerOnEmulatorStopped;
}

// Binary viewer settings
bool ConfigManager::getOpenBinaryViewerOnAssembly()
{
    return m_settings.openBinaryViewerOnAssembly;
}

bool ConfigManager::getOpenBinaryViewerOnEmulatorPaused()
{
    return m_settings.openBinaryViewerOnEmulatorPaused;
}

bool ConfigManager::getOpenBinaryViewerOnEmulatorStopped()
{
    return m_settings.openBinaryViewerOnEmulatorStopped;
}

// Emulator settings
bool ConfigManager::getStartEmulatorPaused()
{
    return m_settings.startEmulatorPaused;
}

bool ConfigManager::getMonitorPlugged()
{
    return m_settings.monitorPlugged;
}

bool ConfigManager::getRTCPlugged()
{
    return m_settings.rtcPlugged;
}

bool ConfigManager::getKeyboardPlugged()
{
    return m_settings.keyboardPlugged;
}

bool ConfigManager::getEepromPlugged()
{
    return m_settings.eepromPlugged;
}

bool ConfigManager::getDismissReassemblyWarnings()
{
    return m_settings.dismissReassemblyWarnings;
}

// IDE settings
QString ConfigManager::getDefaultProjectsPath()
{
    return m_settings.defaultProjectsPath;
}

// PRIVATE
ConfigManager::ConfigManager()
{
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
                            m_settings.tabSize = tabSize;
                    }
                    else if (key == "RAM_AS_DEFAULT_MEMORY_TARGET")
                    {
                        m_settings.ramAsDefaultMemoryTarget = (value == "TRUE");
                    }
                    else if (key == "OPEN_CPU_STATE_VIEWER_EMULATOR_PAUSED")
                    {
                        m_settings.openCpuStateViewerOnEmulatorPaused = (value == "TRUE");
                    }
                    else if (key == "OPEN_CPU_STATE_VIEWER_EMULATOR_STOPPED")
                    {
                        m_settings.openCpuStateViewerOnEmulatorStopped = (value == "TRUE");
                    }
                    else if (key == "OPEN_BIN_VIEWER_ASSEMBLY")
                    {
                        m_settings.openBinaryViewerOnAssembly = (value == "TRUE");
                    }
                    else if (key == "OPEN_BIN_VIEWER_EMULATOR_PAUSED")
                    {
                        m_settings.openBinaryViewerOnEmulatorPaused = (value == "TRUE");
                    }
                    else if (key == "OPEN_BIN_VIEWER_EMULATOR_STOPPPED")
                    {
                        m_settings.openBinaryViewerOnEmulatorStopped = (value == "TRUE");
                    }
                    else if (key == "START_EMULATOR_PAUSED")
                    {
                        m_settings.monitorPlugged = (value == "TRUE");
                    }
                    else if (key == "MONITOR_PLUGGED")
                    {
                        m_settings.monitorPlugged = (value == "TRUE");
                    }
                    else if (key == "RTC_PLUGGED")
                    {
                        m_settings.rtcPlugged = (value == "TRUE");
                    }
                    else if (key == "KEYBOARD_PLUGGED")
                    {
                        m_settings.keyboardPlugged = (value == "TRUE");
                    }
                    else if (key == "EEPROM_PLUGGED")
                    {
                        m_settings.eepromPlugged = (value == "TRUE");
                    }
                    else if (key == "DISMISS_REASSEMBLY_WARNINGS")
                    {
                        m_settings.dismissReassemblyWarnings = (value == "TRUE");
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

        configFile.close();
    }

    // Recent projects file
    QString recentProjectsFilePath(QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation) + "/recentProjects.cfg");
    QFile recentProjectsFile(recentProjectsFilePath);
    m_settings.recentProjects.clear();

    if (QFile::exists(configFilePath))
    {
        if (recentProjectsFile.open(QIODevice::ReadOnly))
        {
            QTextStream in(&recentProjectsFile);

            int i(0);
            while (!in.atEnd() && i < 10)
            {
                QString line = in.readLine();

                m_settings.recentProjects.push_back(line);

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

        out << "TAB_SIZE=" << QString::number(m_settings.tabSize) << "\n";
        out << "RAM_AS_DEFAULT_MEMORY_TARGET=" << (m_settings.ramAsDefaultMemoryTarget ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_CPU_STATE_VIEWER_EMULATOR_PAUSED=" << (m_settings.openCpuStateViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_CPU_STATE_VIEWER_EMULATOR_STOPPED=" << (m_settings.openCpuStateViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_ASSEMBLY=" << (m_settings.openBinaryViewerOnAssembly ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_EMULATOR_PAUSED=" << (m_settings.openBinaryViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_EMULATOR_STOPPPED=" << (m_settings.openBinaryViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";
        out << "START_EMULATOR_PAUSED=" << (m_settings.startEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "MONITOR_PLUGGED=" << (m_settings.monitorPlugged ? "TRUE" : "FALSE") << "\n";
        out << "RTC_PLUGGED=" << (m_settings.rtcPlugged ? "TRUE" : "FALSE") << "\n";
        out << "KEYBOARD_PLUGGED=" << (m_settings.keyboardPlugged ? "TRUE" : "FALSE") << "\n";
        out << "EEPROM_PLUGGED=" << (m_settings.eepromPlugged ? "TRUE" : "FALSE") << "\n";
        out << "DISMISS_REASSEMBLY_WARNINGS=" << (m_settings.dismissReassemblyWarnings ? "TRUE" : "FALSE") << "\n";
        out << "DEFAULT_PROJECT_PATH=" << m_settings.defaultProjectsPath << "\n";

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

        for (unsigned int i(0); i < m_settings.recentProjects.size(); i++)
        {
            out << m_settings.recentProjects[i] << "\n";
        }

        recentProjectsFile.close();
        return true;
    }

    return false;
}

void ConfigManager::flushRecentProjects()
{
    m_settings.recentProjects.removeDuplicates();

    for (auto &p : m_settings.recentProjects)
    {
        if (p.isEmpty())
        {
            m_settings.recentProjects.removeOne(p);
        }

        if (!QFileInfo(p).isFile())
        {
            m_settings.recentProjects.removeOne(p);
        }

        if (QFileInfo(p).completeSuffix() != "hcp") // HBC-2 project files
        {
            m_settings.recentProjects.removeOne(p);
        }
    }
}


// ========= SettingsDialog class =========
SettingsDialog::SettingsDialog(ConfigManager *configManager, QWidget *parent) : QDialog(parent)
{
    m_configManager = configManager;
    setWindowSettings();

    initMenu();

    QHBoxLayout *panelLayout = new QHBoxLayout;
    panelLayout->addWidget(m_menuList);
    panelLayout->addWidget(m_menuWidgets);

    m_closeButton = new QPushButton(tr("Close"), this);
    connect(m_closeButton, SIGNAL(clicked()), this, SLOT(accept()));

    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addLayout(panelLayout);
    mainLayout->addWidget(m_closeButton);

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

void SettingsDialog::ramAsDefaultMemoryTargetChanged()
{
    m_configManager->setRamAsDefaultMemoryTarget(m_ramAsDefaultMemoryTargetCheckBox->isChecked());
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

void SettingsDialog::browseProjectsPathClicked()
{
    QString newDefaultProjectsPath = QFileDialog::getExistingDirectory(this, tr("Select new projects directory"), m_configManager->getDefaultProjectsPath(), QFileDialog::ShowDirsOnly);

    m_defaultProjectsPathLineEdit->setText(newDefaultProjectsPath);
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

// PRIVATE
void SettingsDialog::initMenu()
{
    // Init layouts
    initEditorSettingsLayout();
    initAssemblerSettingsLayout();
    initEmulatorSettingsLayout();
    initCpuStateViewerSettingsLayout();
    initBinaryViewerSettingsLayout();

    // Set the window main layout
    m_menuList = new QListWidget(this);
    addMenu(m_editorSettingsItem, m_editorSettingsWidget);
    addMenu(m_assemblerSettingsItem, m_assemblerSettingsWidget);
    addMenu(m_emulatorSettingsItem, m_emulatorSettingsWidget);
    addMenu(m_cpuStateViewerSettingsItem, m_cpuStateViewerSettingsWidget);
    addMenu(m_binaryViewerSettingsItem, m_binaryViewerSettingsWidget);
    m_menuList->setCurrentRow(0);

    // Stacked widget
    m_menuWidgets = new QStackedWidget(this);
    m_menuWidgets->addWidget(m_editorSettingsWidget);
    m_menuWidgets->addWidget(m_assemblerSettingsWidget);
    m_menuWidgets->addWidget(m_emulatorSettingsWidget);
    m_menuWidgets->addWidget(m_cpuStateViewerSettingsWidget);
    m_menuWidgets->addWidget(m_binaryViewerSettingsWidget);
    m_menuWidgets->setCurrentIndex(0);

    connect(m_menuList, SIGNAL(currentRowChanged(int)), this, SLOT(menuSelected(int)));
}

void SettingsDialog::setWindowSettings()
{
    resize(DIALOG_WIDTH, DIALOG_HEIGHT);
    setWindowTitle(tr("Settings"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));
}

void SettingsDialog::initEditorSettingsLayout()
{
    // Layout init
    m_editorSettingsWidget = new QWidget(this);
    m_editorSettingsLayout = new QVBoxLayout;

    m_editorSettingsItem = new QListWidgetItem(tr("Editor"));
    m_editorSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Editor"), qobject_cast<QWidget*>(m_editorSettingsLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    QLabel *defaultProjectsPathLabel = new QLabel(tr("Projects directory"), qobject_cast<QWidget*>(m_editorSettingsLayout));
    m_browseProjectsPath = new QPushButton(tr("Browse"), qobject_cast<QWidget*>(m_editorSettingsLayout));
    m_defaultProjectsPathLineEdit = new QLineEdit(qobject_cast<QWidget*>(m_editorSettingsLayout));
    m_defaultProjectsPathLineEdit->setText(m_configManager->getDefaultProjectsPath());
    m_defaultProjectsPathLineEdit->setMinimumWidth(250);
    QHBoxLayout *defaultProjectsPathLayout = new QHBoxLayout;
    defaultProjectsPathLayout->addWidget(m_defaultProjectsPathLineEdit);
    defaultProjectsPathLayout->addWidget(m_browseProjectsPath);

    QLabel *tabSizeLabel = new QLabel(tr("Tab size"), qobject_cast<QWidget*>(m_editorSettingsLayout));
    m_tabSizeSpinBox = new QSpinBox(qobject_cast<QWidget*>(m_editorSettingsLayout));
    m_tabSizeSpinBox->setSuffix(" spaces");
    m_tabSizeSpinBox->setRange(1, 16);
    m_tabSizeSpinBox->setValue(m_configManager->getTabSize());
    QHBoxLayout *tabSizeLayout = new QHBoxLayout;
    tabSizeLayout->addWidget(tabSizeLabel);
    tabSizeLayout->addWidget(m_tabSizeSpinBox);
    // ------------------

    // Final layout configuration
    m_editorSettingsLayout->addWidget(mainLabel);
    m_editorSettingsLayout->addWidget(defaultProjectsPathLabel);
    m_editorSettingsLayout->addLayout(defaultProjectsPathLayout);
    m_editorSettingsLayout->addLayout(tabSizeLayout);
    m_editorSettingsLayout->addStretch();
    m_editorSettingsWidget->setLayout(m_editorSettingsLayout);

    // Connections
    connect(m_tabSizeSpinBox, SIGNAL(valueChanged(int)), this, SLOT(tabSizeChanged(int)));
    connect(m_browseProjectsPath, SIGNAL(clicked()), this, SLOT(browseProjectsPathClicked()));
    connect(m_defaultProjectsPathLineEdit, SIGNAL(textChanged(QString)), this, SLOT(defaultProjectsPathChanged(QString)));
}

void SettingsDialog::initAssemblerSettingsLayout()
{
    // Layout init
    m_assemblerSettingsWidget = new QWidget(this);
    m_assemblerSettingsLayout = new QVBoxLayout;

    m_assemblerSettingsItem = new QListWidgetItem(tr("Assembler"));
    m_assemblerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Assembler"), qobject_cast<QWidget*>(m_assemblerSettingsLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_ramAsDefaultMemoryTargetCheckBox = new QCheckBox(tr("RAM as default memory target"), qobject_cast<QWidget*>(m_assemblerSettingsLayout));
    m_ramAsDefaultMemoryTargetCheckBox->setChecked(m_configManager->getRamAsDefaultMemoryTarget());
    // ------------------

    // Final layout configuration
    m_assemblerSettingsLayout->addWidget(mainLabel);
    m_assemblerSettingsLayout->addWidget(m_ramAsDefaultMemoryTargetCheckBox);
    m_assemblerSettingsLayout->addStretch();
    m_assemblerSettingsWidget->setLayout(m_assemblerSettingsLayout);

    // Connections
    connect(m_ramAsDefaultMemoryTargetCheckBox, SIGNAL(stateChanged(int)), this, SLOT(ramAsDefaultMemoryTargetChanged()));
}

void SettingsDialog::initEmulatorSettingsLayout()
{
    // Layout init
    m_emulatorSettingsWidget = new QWidget(this);
    m_emulatorSettingsLayout = new QVBoxLayout;

    m_emulatorSettingsItem = new QListWidgetItem(tr("Emulator"));
    m_emulatorSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Emulator"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_startPausedCheckBox = new QCheckBox(tr("Start paused"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    m_startPausedCheckBox->setChecked(m_configManager->getStartEmulatorPaused());

    m_plugMonitorCheckBox = new QCheckBox(tr("Monitor plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    m_plugMonitorCheckBox->setChecked(m_configManager->getMonitorPlugged());

    m_plugRTCCheckBox = new QCheckBox(tr("Real Time Clock plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    m_plugRTCCheckBox->setChecked(m_configManager->getRTCPlugged());

    m_plugKeyboardCheckBox = new QCheckBox(tr("Keyboard plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    m_plugKeyboardCheckBox->setChecked(m_configManager->getKeyboardPlugged());

    m_plugEepromCheckBox = new QCheckBox(tr("EEPROM plugged-in by default"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    m_plugEepromCheckBox->setChecked(m_configManager->getEepromPlugged());

    m_dismissReassemblyWarningsCheckBox = new QCheckBox(tr("Dismiss warnings when running an unassembled project"), qobject_cast<QWidget*>(m_emulatorSettingsLayout));
    m_dismissReassemblyWarningsCheckBox->setChecked(m_configManager->getDismissReassemblyWarnings());
    // ------------------

    // Final layout configuration
    m_emulatorSettingsLayout->addWidget(mainLabel);
    m_emulatorSettingsLayout->addWidget(m_startPausedCheckBox);
    m_emulatorSettingsLayout->addWidget(m_plugMonitorCheckBox);
    m_emulatorSettingsLayout->addWidget(m_plugRTCCheckBox);
    m_emulatorSettingsLayout->addWidget(m_plugKeyboardCheckBox);
    m_emulatorSettingsLayout->addWidget(m_plugEepromCheckBox);
    m_emulatorSettingsLayout->addWidget(m_dismissReassemblyWarningsCheckBox);
    m_emulatorSettingsLayout->addStretch();
    m_emulatorSettingsWidget->setLayout(m_emulatorSettingsLayout);

    // Connections
    connect(m_startPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(startPausedChanged()));
    connect(m_plugMonitorCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugMonitorChanged()));
    connect(m_plugRTCCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugRTCChanged()));
    connect(m_plugKeyboardCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugKeyboardChanged()));
    connect(m_plugEepromCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugEepromChanged()));
    connect(m_dismissReassemblyWarningsCheckBox, SIGNAL(stateChanged(int)), this, SLOT(dismissReassemblyWarningsChanged()));
}

void SettingsDialog::initCpuStateViewerSettingsLayout()
{
    // Layout init
    m_cpuStateViewerSettingsWidget = new QWidget(this);
    m_cpuStateViewerSettingsLayout = new QVBoxLayout;

    m_cpuStateViewerSettingsItem = new QListWidgetItem(tr("CPU State Viewer"));
    m_cpuStateViewerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("CPU State Viewer"), qobject_cast<QWidget*>(m_cpuStateViewerSettingsLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_openCpuStateViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), qobject_cast<QWidget*>(m_cpuStateViewerSettingsLayout));
    m_openCpuStateViewerOnEmulatorPausedCheckBox->setChecked(m_configManager->getOpenCpuStateViewerOnEmulatorPaused());

    m_openCpuStateViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), qobject_cast<QWidget*>(m_cpuStateViewerSettingsLayout));
    m_openCpuStateViewerOnEmulatorStoppedCheckBox->setChecked(m_configManager->getOpenCpuStateViewerOnEmulatorStopped());
    // ------------------

    // Final layout configuration
    m_cpuStateViewerSettingsLayout->addWidget(mainLabel);
    m_cpuStateViewerSettingsLayout->addWidget(m_openCpuStateViewerOnEmulatorPausedCheckBox);
    m_cpuStateViewerSettingsLayout->addWidget(m_openCpuStateViewerOnEmulatorStoppedCheckBox);
    m_cpuStateViewerSettingsLayout->addStretch();
    m_cpuStateViewerSettingsWidget->setLayout(m_cpuStateViewerSettingsLayout);

    // Connections
    connect(m_openCpuStateViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openCpuStateViewerOnEmulatorPausedChanged()));
    connect(m_openCpuStateViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openCpuStateViewerOnEmulatorStoppedChanged()));
}

void SettingsDialog::initBinaryViewerSettingsLayout()
{
    // Layout init
    m_binaryViewerSettingsWidget = new QWidget(this);
    m_binaryViewerSettingsLayout = new QVBoxLayout;

    m_binaryViewerSettingsItem = new QListWidgetItem(tr("Binary Viewer"));
    m_binaryViewerSettingsItem->setSizeHint(QSize(0, ITEM_HEIGHT));

    // ----  Widgets ----
    QLabel *mainLabel = new QLabel(tr("Binary Viewer"), qobject_cast<QWidget*>(m_binaryViewerSettingsLayout));
    mainLabel->setStyleSheet("font-weight: bold;");

    m_openBinaryViewerOnAssemblyCheckBox = new QCheckBox(tr("Open viewer after each assembly"), qobject_cast<QWidget*>(m_binaryViewerSettingsLayout));
    m_openBinaryViewerOnAssemblyCheckBox->setChecked(m_configManager->getOpenBinaryViewerOnAssembly());

    m_openBinaryViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), qobject_cast<QWidget*>(m_binaryViewerSettingsLayout));
    m_openBinaryViewerOnEmulatorPausedCheckBox->setChecked(m_configManager->getOpenBinaryViewerOnEmulatorPaused());

    m_openBinaryViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), qobject_cast<QWidget*>(m_binaryViewerSettingsLayout));
    m_openBinaryViewerOnEmulatorStoppedCheckBox->setChecked(m_configManager->getOpenBinaryViewerOnEmulatorStopped());
    // ------------------

    // Final layout configuration
    m_binaryViewerSettingsLayout->addWidget(mainLabel);
    m_binaryViewerSettingsLayout->addWidget(m_openBinaryViewerOnAssemblyCheckBox);
    m_binaryViewerSettingsLayout->addWidget(m_openBinaryViewerOnEmulatorPausedCheckBox);
    m_binaryViewerSettingsLayout->addWidget(m_openBinaryViewerOnEmulatorStoppedCheckBox);
    m_binaryViewerSettingsLayout->addStretch();
    m_binaryViewerSettingsWidget->setLayout(m_binaryViewerSettingsLayout);

    // Connections
    connect(m_openBinaryViewerOnAssemblyCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnAssemblyChanged()));
    connect(m_openBinaryViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnEmulatorPausedChanged()));
    connect(m_openBinaryViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnEmulatorStoppedChanged()));
}

void SettingsDialog::addMenu(QListWidgetItem *newMenuItem, QWidget *newMenuWidget)
{
    m_menuList->addItem(newMenuItem);
    m_menuWidgetsList.push_back(newMenuWidget);
}
