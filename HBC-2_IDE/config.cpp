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

// SETTERS
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

                    if (key == "OPEN_CPU_STATE_VIEWER_EMULATOR_PAUSED")
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

        out << "OPEN_CPU_STATE_VIEWER_EMULATOR_PAUSED=" << (m_settings.openCpuStateViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_CPU_STATE_VIEWER_EMULATOR_STOPPED=" << (m_settings.openCpuStateViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_ASSEMBLY=" << (m_settings.openBinaryViewerOnAssembly ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_EMULATOR_PAUSED=" << (m_settings.openBinaryViewerOnEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "OPEN_BIN_VIEWER_EMULATOR_STOPPPED=" << (m_settings.openBinaryViewerOnEmulatorStopped ? "TRUE" : "FALSE") << "\n";
        out << "START_EMULATOR_PAUSED=" << (m_settings.startEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "MONITOR_PLUGGED=" << (m_settings.monitorPlugged ? "TRUE" : "FALSE") << "\n";
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
            out << m_settings.recentProjects[i];
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


// SettingsDialog class
SettingsDialog::SettingsDialog(ConfigManager *configManager, QWidget *parent) : QDialog(parent)
{
    m_configManager = configManager;

    setWindowTitle(tr("Settings"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    // IDE settings
    QGroupBox *ideSettingsGroupBox = new QGroupBox(tr("Editor"), this);

    QLabel *defaultProjectsPathLabel = new QLabel(tr("Projects directory"), this);
    QPushButton *browseProjectsPath = new QPushButton(tr("Browse"), this);
    m_defaultProjectsPathLineEdit = new QLineEdit(this);
    m_defaultProjectsPathLineEdit->setText(configManager->getDefaultProjectsPath());
    m_defaultProjectsPathLineEdit->setMinimumWidth(250);
    QHBoxLayout *defaultProjectsPathLayout = new QHBoxLayout;
    defaultProjectsPathLayout->addWidget(m_defaultProjectsPathLineEdit);
    defaultProjectsPathLayout->addWidget(browseProjectsPath);

    QVBoxLayout *ideSettingsLayout = new QVBoxLayout;
    ideSettingsLayout->addWidget(defaultProjectsPathLabel);
    ideSettingsLayout->addLayout(defaultProjectsPathLayout);
    ideSettingsGroupBox->setLayout(ideSettingsLayout);


    // Cpu state viewer settings
    QGroupBox *cpuStateViewerSettingsGroupBox = new QGroupBox(tr("CPU status viewer"), this);

    m_openCpuStateViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), this);
    m_openCpuStateViewerOnEmulatorPausedCheckBox->setChecked(configManager->getOpenCpuStateViewerOnEmulatorPaused());

    m_openCpuStateViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), this);
    m_openCpuStateViewerOnEmulatorStoppedCheckBox->setChecked(configManager->getOpenCpuStateViewerOnEmulatorStopped());

    QVBoxLayout *cpuStateViewerSettingsLayout = new QVBoxLayout;
    cpuStateViewerSettingsLayout->addWidget(m_openCpuStateViewerOnEmulatorPausedCheckBox);
    cpuStateViewerSettingsLayout->addWidget(m_openCpuStateViewerOnEmulatorStoppedCheckBox);
    cpuStateViewerSettingsGroupBox->setLayout(cpuStateViewerSettingsLayout);


    // Binary viewer settings
    QGroupBox *binaryViewerSettingsGroupBox = new QGroupBox(tr("Binary viewer"), this);

    m_openBinaryViewerOnAssemblyCheckBox = new QCheckBox(tr("Open viewer after each assembly"), this);
    m_openBinaryViewerOnAssemblyCheckBox->setChecked(configManager->getOpenBinaryViewerOnAssembly());

    m_openBinaryViewerOnEmulatorPausedCheckBox = new QCheckBox(tr("Open viewer when the emulator is paused"), this);
    m_openBinaryViewerOnEmulatorPausedCheckBox->setChecked(configManager->getOpenBinaryViewerOnEmulatorPaused());

    m_openBinaryViewerOnEmulatorStoppedCheckBox = new QCheckBox(tr("Open viewer when the emulator is stopped"), this);
    m_openBinaryViewerOnEmulatorStoppedCheckBox->setChecked(configManager->getOpenBinaryViewerOnEmulatorStopped());

    QVBoxLayout *binaryViewerSettingsLayout = new QVBoxLayout;
    binaryViewerSettingsLayout->addWidget(m_openBinaryViewerOnAssemblyCheckBox);
    binaryViewerSettingsLayout->addWidget(m_openBinaryViewerOnEmulatorPausedCheckBox);
    binaryViewerSettingsLayout->addWidget(m_openBinaryViewerOnEmulatorStoppedCheckBox);
    binaryViewerSettingsGroupBox->setLayout(binaryViewerSettingsLayout);


    // Emulator settings
    QGroupBox *emulatorSettingsGroupBox = new QGroupBox(tr("Emulator"), this);

    m_startPausedCheckBox = new QCheckBox(tr("Start paused"), this);
    m_startPausedCheckBox->setChecked(configManager->getStartEmulatorPaused());

    m_plugMonitorCheckBox = new QCheckBox(tr("Monitor plugged-in by default"), this);
    m_plugMonitorCheckBox->setChecked(configManager->getMonitorPlugged());

    m_dismissReassemblyWarningsCheckBox = new QCheckBox(tr("Dismiss warnings when running an unassembled project"), this);
    m_dismissReassemblyWarningsCheckBox->setChecked(configManager->getDismissReassemblyWarnings());

    QVBoxLayout *emulatorSettingsLayout = new QVBoxLayout;
    emulatorSettingsLayout->addWidget(m_startPausedCheckBox);
    emulatorSettingsLayout->addWidget(m_plugMonitorCheckBox);
    emulatorSettingsLayout->addWidget(m_dismissReassemblyWarningsCheckBox);
    emulatorSettingsGroupBox->setLayout(emulatorSettingsLayout);


    QPushButton *closeButton = new QPushButton(tr("Close"), this);


    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(cpuStateViewerSettingsGroupBox);
    mainLayout->addWidget(binaryViewerSettingsGroupBox);
    mainLayout->addWidget(ideSettingsGroupBox);
    mainLayout->addWidget(emulatorSettingsGroupBox);
    mainLayout->addWidget(closeButton);

    setLayout(mainLayout);


    // Connections
    connect(m_openCpuStateViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openCpuStateViewerOnEmulatorPausedChanged()));
    connect(m_openCpuStateViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openCpuStateViewerOnEmulatorStoppedChanged()));
    connect(m_openBinaryViewerOnAssemblyCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnAssemblyChanged()));
    connect(m_openBinaryViewerOnEmulatorPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnEmulatorPausedChanged()));
    connect(m_openBinaryViewerOnEmulatorStoppedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(openBinaryViewerOnEmulatorStoppedChanged()));
    connect(m_startPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(startPausedChanged()));
    connect(m_plugMonitorCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugMonitorChanged()));
    connect(m_dismissReassemblyWarningsCheckBox, SIGNAL(stateChanged(int)), this, SLOT(dismissReassemblyWarningsChanged()));
    connect(browseProjectsPath, SIGNAL(clicked()), this, SLOT(browseProjectsPathClicked()));
    connect(m_defaultProjectsPathLineEdit, SIGNAL(textChanged(QString)), this, SLOT(defaultProjectsPathChanged(QString)));

    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
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
