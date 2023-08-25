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
    m_singleton = nullptr;
}

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

bool ConfigManager::setDefaultProjectsPath(QString defaultPath)
{
    if (defaultPath.isEmpty())
        return false;

    m_settings.defaultProjectsPath = defaultPath;
    saveConfigFile();

    return true;
}

bool ConfigManager::getStartEmulatorPaused()
{
    return m_settings.startEmulatorPaused;
}

bool ConfigManager::getMonitorPlugged()
{
    return m_settings.monitorPlugged;
}

QString ConfigManager::getDefaultProjectsPath()
{
    return m_settings.defaultProjectsPath;
}

// PRIVATE
ConfigManager::ConfigManager()
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

        out << "START_EMULATOR_PAUSED=" << (m_settings.startEmulatorPaused ? "TRUE" : "FALSE") << "\n";
        out << "MONITOR_PLUGGED=" << (m_settings.monitorPlugged ? "TRUE" : "FALSE") << "\n";
        out << "DEFAULT_PROJECT_PATH=" << m_settings.defaultProjectsPath << "\n";

        return true;
    }

    return false;
}


// SettingsDialog class
SettingsDialog::SettingsDialog(ConfigManager *configManager, QWidget *parent) : QDialog(parent)
{
    m_configManager = configManager;

    setWindowTitle(tr("Settings"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    // Editor settings
    QGroupBox *editorSettingsGroupBox = new QGroupBox(tr("Editor"), this);

    QLabel *defaultProjectsPathLabel = new QLabel(tr("Projects directory"), this);
    QPushButton *browseProjectsPath = new QPushButton(tr("Browse"), this);
    m_defaultProjectsPathLineEdit = new QLineEdit(this);
    m_defaultProjectsPathLineEdit->setText(configManager->getDefaultProjectsPath());
    m_defaultProjectsPathLineEdit->setMinimumWidth(250);
    QHBoxLayout *defaultProjectsPathLayout = new QHBoxLayout;
    defaultProjectsPathLayout->addWidget(m_defaultProjectsPathLineEdit);
    defaultProjectsPathLayout->addWidget(browseProjectsPath);

    QVBoxLayout *editorSettingsLayout = new QVBoxLayout;
    editorSettingsLayout->addWidget(defaultProjectsPathLabel);
    editorSettingsLayout->addLayout(defaultProjectsPathLayout);
    editorSettingsGroupBox->setLayout(editorSettingsLayout);


    // Emulator settings
    QGroupBox *emulatorSettingsGroupBox = new QGroupBox(tr("Emulator"), this);

    m_startPausedCheckBox = new QCheckBox(tr("Start the emulator paused"), this);
    m_startPausedCheckBox->setChecked(configManager->getStartEmulatorPaused());

    m_plugMonitorCheckBox = new QCheckBox(tr("Monitor plugged-in by default"), this);
    m_plugMonitorCheckBox->setChecked(configManager->getMonitorPlugged());

    QVBoxLayout *emulatorSettingsLayout = new QVBoxLayout;
    emulatorSettingsLayout->addWidget(m_startPausedCheckBox);
    emulatorSettingsLayout->addWidget(m_plugMonitorCheckBox);
    emulatorSettingsGroupBox->setLayout(emulatorSettingsLayout);


    QPushButton *closeButton = new QPushButton(tr("Close"), this);


    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addWidget(editorSettingsGroupBox);
    mainLayout->addWidget(emulatorSettingsGroupBox);
    mainLayout->addWidget(closeButton);

    setLayout(mainLayout);


    // Connections
    connect(m_startPausedCheckBox, SIGNAL(stateChanged(int)), this, SLOT(startPausedChanged()));
    connect(m_plugMonitorCheckBox, SIGNAL(stateChanged(int)), this, SLOT(plugMonitorChanged()));
    connect(browseProjectsPath, SIGNAL(clicked()), this, SLOT(browseProjectsPathClicked()));
    connect(m_defaultProjectsPathLineEdit, SIGNAL(textChanged(const QString&)), this, SLOT(defaultProjectsPathChanged()));

    connect(closeButton, SIGNAL(clicked()), this, SLOT(accept()));
}

void SettingsDialog::startPausedChanged()
{
    m_configManager->setStartEmulatorPaused(m_startPausedCheckBox->isChecked());
}

void SettingsDialog::plugMonitorChanged()
{
    m_configManager->setMonitorPlugged(m_plugMonitorCheckBox->isChecked());
}

void SettingsDialog::browseProjectsPathClicked()
{
    QString newDefaultProjectsPath = QFileDialog::getExistingDirectory(this, tr("Select new projects directory"), m_configManager->getDefaultProjectsPath(), QFileDialog::ShowDirsOnly);

    m_defaultProjectsPathLineEdit->setText(newDefaultProjectsPath);
}

void SettingsDialog::defaultProjectsPathChanged()
{
    if (m_defaultProjectsPathLineEdit->text().isEmpty())
    {
        QMessageBox::warning(this, tr("Invalid projects directory"), tr("The projects directory cannot be empty")); // Cannot be empty
    }
    else
    {
        m_configManager->setDefaultProjectsPath(m_defaultProjectsPathLineEdit->text());
    }
}
