#include "projectManager.h"

#include <QVBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QCheckBox>

// === NEW PROJECT DIALOG CLASS ===
NewProjectDialog::NewProjectDialog(bool &ok, QString &projectPath,
                                   bool &includeInterruptHandler, bool &includeMathLibrary,
                                   ConfigManager *configManager, QWidget *parent)
    : QDialog(parent), m_ok(ok), m_projectPath(projectPath), m_includeInterruptHandler(includeInterruptHandler), m_includeMathLibrary(includeMathLibrary)
{
    setWindowTitle(tr("Create a new project"));
    setWindowIcon(QIcon(":/icons/res/logo.png"));

    // Widgets
    QLabel *projectNameLabel = new QLabel(tr("Project name"), this);
    m_projectNameLineEdit = new QLineEdit(this);
    m_projectNameLineEdit->setText(tr("newProject"));
    QHBoxLayout *projectNameLayout = new QHBoxLayout;
    projectNameLayout->addWidget(projectNameLabel);
    projectNameLayout->addWidget(m_projectNameLineEdit);

    QLabel *projectDirectoryLabel = new QLabel(tr("Project directory"), this);
    m_projectDirectoryLineEdit = new QLineEdit(this);
    m_projectDirectoryLineEdit->setText(configManager->getDefaultProjectsPath());
    QPushButton *projectDirectoryBrowseButton = new QPushButton(tr("Browse"), this);
    QHBoxLayout *projectDirectoryLayout = new QHBoxLayout;
    projectDirectoryLayout->addWidget(projectDirectoryLabel);
    projectDirectoryLayout->addWidget(m_projectDirectoryLineEdit);
    projectDirectoryLayout->addWidget(projectDirectoryBrowseButton);

    m_createSubdirectoryCheckBox = new QCheckBox(tr("Create a subdirectory for the project"), this);
    m_createSubdirectoryCheckBox->setChecked(true);

    QFrame *separator = new QFrame(this);
    separator->setFrameShape(QFrame::HLine);
    separator->setFrameShadow(QFrame::Sunken);

    m_includeInterruptHandlerCheckBox = new QCheckBox(tr("Include a default interrupt handler for plugged devices"), this);
    m_includeInterruptHandlerCheckBox->setChecked(true);

    m_includeMathLibraryCheckBox = new QCheckBox(tr("Include the default math library"), this);
    m_includeMathLibraryCheckBox->setChecked(/*true*/false); // TODO: set enabled by default when the library is finished
    m_includeMathLibraryCheckBox->setCheckable(false); // TODO: remove when adding this library is possible

    m_createButton = new QPushButton(tr("Create"), this);
    m_cancelButton = new QPushButton(tr("Cancel"), this);
    QHBoxLayout *buttonsLayout = new QHBoxLayout;
    buttonsLayout->addWidget(m_createButton);
    buttonsLayout->addWidget(m_cancelButton);

    // Layout
    QVBoxLayout *mainLayout = new QVBoxLayout;
    mainLayout->addLayout(projectNameLayout);
    mainLayout->addLayout(projectDirectoryLayout);
    mainLayout->addWidget(m_createSubdirectoryCheckBox);
    mainLayout->addWidget(separator);
    mainLayout->addWidget(m_includeInterruptHandlerCheckBox);
    mainLayout->addWidget(m_includeMathLibraryCheckBox);
    mainLayout->addLayout(buttonsLayout);

    setLayout(mainLayout);

    // Connections
    connect(projectDirectoryBrowseButton, SIGNAL(clicked()), this, SLOT(browse()));
    connect(m_projectNameLineEdit, SIGNAL(textChanged(QString)), this, SLOT(checkNameAndDirectoryValidity()));
    connect(m_projectDirectoryLineEdit, SIGNAL(textChanged(QString)), this, SLOT(checkNameAndDirectoryValidity()));
    connect(m_createButton, SIGNAL(clicked()), this, SLOT(accept()));
    connect(m_cancelButton, SIGNAL(clicked()), this, SLOT(reject()));
}

void NewProjectDialog::browse()
{
    QString newProjectDirectory = QFileDialog::getExistingDirectory(this, tr("Select new project directory"), m_projectDirectoryLineEdit->text());

    if (!newProjectDirectory.isEmpty())
    {
        m_projectDirectoryLineEdit->setText(newProjectDirectory);
        checkNameAndDirectoryValidity();
    }
}

void NewProjectDialog::checkNameAndDirectoryValidity()
{
    if (m_projectNameLineEdit->text().isEmpty() || m_projectDirectoryLineEdit->text().isEmpty())
    {
        m_createButton->setEnabled(false);
    }
    else
    {
        m_createButton->setEnabled(true);
    }
}

void NewProjectDialog::accept()
{
    m_ok = true;

    m_projectPath = m_projectDirectoryLineEdit->text() + "/";

    if (m_createSubdirectoryCheckBox->isChecked())
    {
        if (QDir(m_projectDirectoryLineEdit->text() + "/" + m_projectNameLineEdit->text()).exists())
        {
            QMessageBox::warning(this, tr("Project already exists"), tr("A directory with the same name already exists"));
            reject();
        }

        QDir directory;
        directory.mkdir(m_projectDirectoryLineEdit->text() + "/" + m_projectNameLineEdit->text());

        m_projectPath += m_projectNameLineEdit->text() + "/";
    }
    else
    {
        if (QFile(m_projectDirectoryLineEdit->text() + "/" + m_projectNameLineEdit->text() + ((m_projectNameLineEdit->text().indexOf("*.hcp") == -1) ? ".hcp" : "")).exists())
        {
            QMessageBox::warning(this, tr("Project already exists"), tr("A project file with the same name already exists"));
            reject();
        }
    }

    m_projectPath += m_projectNameLineEdit->text() + ((m_projectNameLineEdit->text().indexOf("*.hcp") == -1) ? ".hcp" : "");

    m_includeInterruptHandler = m_includeInterruptHandlerCheckBox->isChecked();
    m_includeMathLibrary = m_includeMathLibraryCheckBox->isChecked();

    QDialog::accept();
}

void NewProjectDialog::reject()
{
    m_ok = false;
    QDialog::reject();
}

// === PROJECT NODE CLASS ===
ProjectItem::ProjectItem(QString path, QString associatedProjectName, ProjectItem *parent, bool isFolder) : QTreeWidgetItem(parent)
{
    QFileInfo info(path);

    m_isFolder = isFolder;
    m_path = path;
    m_name = m_isFolder ? info.baseName() : info.fileName();
    m_associatedProjectName = associatedProjectName;

    setText(0, m_name);
}

ProjectItem::ProjectItem(QString projectName) : QTreeWidgetItem()
{
    m_isFolder = true;
    m_path = "";
    m_name = projectName;
    m_associatedProjectName = projectName;

    setText(0, projectName);
}

ProjectItem::~ProjectItem()
{}

ProjectItem* ProjectItem::addFolder(QString name)
{
    if (name == "")
        return nullptr;

    QString path(m_path + "/" + name);

    ProjectItem* child = new ProjectItem(path, m_associatedProjectName, this, true);
    addChild(child);

    return child;
}

ProjectItem* ProjectItem::addFile(QString name)
{
    if (name == "")
        return nullptr;

    if (!name.contains(".has"))
        name += ".has";

    QString path(m_path + "/" + name);

    ProjectItem* child = new ProjectItem(path, m_associatedProjectName, this, false);
    addChild(child);

    return child;
}

QString ProjectItem::getName()
{
    return m_name;
}

QString ProjectItem::getPath()
{
    return m_path;
}

QList<QString> ProjectItem::getFilesNamesList()
{
    QList<QString> filesNames;

    if (isFolder())
    {
        for (int i(0); i < childCount(); i++)
        {
            ProjectItem *c = dynamic_cast<ProjectItem*>(child(i));

            filesNames.append(c->getFilesNamesList());
        }
    }
    else
        filesNames.push_back(getName());

    return filesNames;
}

bool ProjectItem::isFolder()
{
    return m_isFolder;
}

bool ProjectItem::isMainNode()
{
    return parent() == nullptr;
}

bool ProjectItem::isPartOf(ProjectItem* item)
{
    for (int i(0); i < childCount(); i++)
    {
        ProjectItem* c = dynamic_cast<ProjectItem*>(child(i));

        if (c == item)
            return true;

        if (c->isPartOf(item))
            return true;
    }

    return false;
}

QList<QString> ProjectItem::getFilesPaths(QString projectPath)
{
    QList<QString> paths;

    if (!m_isFolder)
        paths.push_back(projectPath + getPath());
    else
    {
        for (int i(0); i < childCount(); i++)
        {
            ProjectItem *c = dynamic_cast<ProjectItem*>(child(i));

            paths.append(c->getFilesPaths(projectPath));
        }
    }

    return paths;
}

QList<ProjectItem*> ProjectItem::getFilesItems()
{
    QList<ProjectItem*> filesItems;

    if (!m_isFolder)
    {
        filesItems.push_back(this);
    }
    else
    {
        for (unsigned int i(0); i < childCount(); i++)
        {
            ProjectItem *c(dynamic_cast<ProjectItem*>(child(i)));

            filesItems.append(c->getFilesItems());
        }
    }

    return filesItems;
}

ProjectItem* ProjectItem::getProjectItemFromPath(QString fullPath, QString projectPath)
{
    if (!m_isFolder)
    {
        if (projectPath + getPath() == fullPath)
        {
            return this;
        }
        else
        {
            return nullptr;
        }
    }
    else
    {
        for (int i(0); i < childCount(); i++)
        {
            ProjectItem *c = dynamic_cast<ProjectItem*>(child(i));

            if (c->getProjectItemFromPath(fullPath, projectPath) != nullptr)
            {
                return c;
            }
        }

        return nullptr;
    }
}

QString ProjectItem::getAssociatedProjectName()
{
    return m_associatedProjectName;
}

bool ProjectItem::setPath(QString newPath)
{
    if (!newPath.isEmpty())
    {
        m_path = newPath;

        return true;
    }

    return false;
}

void ProjectItem::setAssociatedProjectName(QString associatedProjectName)
{
    m_associatedProjectName = associatedProjectName;
}

bool ProjectItem::rename(QString newName, QString currentFullPath)
{
    if (!m_name.isEmpty())
    {
        QString oldPath(m_path);
        QFileInfo pathInfo(m_path);
        QFileInfo fullPathInfo(currentFullPath);

        m_name = newName;
        setText(0, m_name);

        QString newPath = pathInfo.absolutePath() + "/" + newName;
        QString newFullPath = fullPathInfo.absolutePath() + "/" + newName;

        QFile::rename(currentFullPath, newFullPath);
        m_path = newPath;

        if (m_isFolder)
        {
            renameParentInChildrenPaths(oldPath, m_path);
        }

        return false;
    }

    return true;
}

void ProjectItem::renameParentInChildrenPaths(QString oldPartialParentPath, QString newPartialParentPath)
{
    for (unsigned int i(0); i < childCount(); i++)
    {
        ProjectItem *c(dynamic_cast<ProjectItem*>(child(i)));
        int oldPartialParentPathPos(c->getPath().indexOf(oldPartialParentPath));

        c->setPath(c->getPath().replace(oldPartialParentPathPos, oldPartialParentPath.size(), newPartialParentPath));

        if (c->isFolder())
        {
            c->renameParentInChildrenPaths(oldPartialParentPath, newPartialParentPath);
        }
    }
}

bool ProjectItem::contains(QString path, QString projectPath)
{
    if ((QFileInfo(projectPath).path() + m_path) == path)
    {
        return true;
    }
    else
    {
        for (unsigned int i(0); i < childCount(); i++)
        {
            ProjectItem *c(dynamic_cast<ProjectItem*>(child(i)));

            if (c->contains(path, projectPath))
            {
                return true;
            }
        }
    }

    return false;
}


// === PROJECT CLASS ===
Project::Project(QString name, QString path, ProjectItem* mainNode)
{
    m_name = name;
    m_path = path;
    m_assembled = false;

    if (mainNode == nullptr)
    {
        m_topItem = new ProjectItem(name);
        addFolder("Source files");
        addFile("main.has", "/Source files");
        addFolder("Libraries");
    }
    else
    {
        m_topItem = mainNode;
        m_topItem->setAssociatedProjectName(m_name);
    }
}

Project::~Project()
{
    delete m_topItem;
}

bool Project::addFolder(QString name, QString path)
{
    QList<QString> parents;
    ProjectItem *nodeToUpdate(m_topItem);
    ProjectItem *temp;
    bool found(false);

    if (name == "" || path == "")
        return false;

    if (path.at(0) != '/')
        path = "/" + path;

    if (path.at(path.size() - 1) != '/')
        path += "/";

    // Deconstruct path
    parents = getPathParentsList(path);

    for (int i(0); i < parents.count(); i++)
    {
        found = false;

        for (int j(0); j < m_topItem->childCount(); j++)
        {
            temp = dynamic_cast<ProjectItem*>(m_topItem->child(j));

            if (temp->getName() == parents.at(i))
            {
                found = true;
                nodeToUpdate = temp;
            }
        }

        if (!found)
            return false; // One of parents node hasn't been found, invalid path
    }

    // Check if the node already contains such a folder name
    for (int i(0); i < nodeToUpdate->childCount(); i++)
    {
        temp = dynamic_cast<ProjectItem*>(nodeToUpdate->child(i));

        if (temp->getName() == name)
            return false;
    }

    // Create new folder
    nodeToUpdate->addFolder(name);

    return true;
}

bool Project::addFile(QString name, QString path)
{
    QList<QString> parents;
    ProjectItem *nodeToUpdate(m_topItem);
    ProjectItem *temp;
    bool found(false);

    if (name == "" || path == "")
        return false;

    if (!name.contains(".has"))
        name += ".has";

    if (path.at(0) != '/')
        path = "/" + path;

    if (path.at(path.size() - 1) != '/')
        path += "/";

    // Deconstruct path
    parents = getPathParentsList(path);

    for (int i(0); i < parents.count(); i++)
    {
        found = false;

        for (int j(0); j < m_topItem->childCount(); j++)
        {
            temp = dynamic_cast<ProjectItem*>(m_topItem->child(j));

            if (temp->getName() == parents.at(i))
            {
                found = true;
                nodeToUpdate = temp;
            }
        }

        if (!found)
            return false; // One of parents node hasn't been found, invalid path
    }

    // Check if the node already contains such a file name
    for (int i(0); i < nodeToUpdate->childCount(); i++)
    {
        temp = dynamic_cast<ProjectItem*>(nodeToUpdate->child(i));

        if (temp->getName() == name)
            return false;
    }

    // Create a new file
    nodeToUpdate->addFile(name);

    return true;
}

ProjectItem* Project::getProjectItemFromFullPath(QString fullPath)
{
    return m_topItem->getProjectItemFromPath(fullPath, getDirPath());
}

QList<QString> Project::getPathParentsList(QString path)
{
    QList<QString> parents;
    QFileInfo info(path);
    QString parentPath(info.path());
    QFileInfo parentInfo(parentPath);

    while (parentInfo.fileName() != "")
    {
        parents.push_front(parentInfo.fileName());

        path = parentPath;
        info.setFile(path);
        parentPath = info.path();
        parentInfo.setFile(parentPath);
    }

    return parents;
}

QString Project::getName()
{
    return m_name;
}

QString Project::getDirPath()
{
    return QFileInfo(m_path).path();
}

QString Project::getPath()
{
    return m_path;
}

bool Project::getAssembled()
{
    return m_assembled;
}

ProjectItem* Project::getTopItem()
{
    return m_topItem;
}

bool Project::isPartOf(ProjectItem* item)
{
    if (m_topItem == item)
        return true;

    return m_topItem->isPartOf(item);
}

QList<QString> Project::getFilesPaths()
{
    return m_topItem->getFilesPaths(getDirPath());
}

QList<QString> Project::getFilesNames()
{
    return m_topItem->getFilesNamesList();
}

bool Project::contains(QString path)
{
    return m_topItem->contains(path, m_path);
}

void Project::setAssembled(bool assembled)
{
    m_assembled = assembled;
}


// === PROJECT MANAGER CLASS ===
ProjectManager* ProjectManager::m_singleton = nullptr;

ProjectManager* ProjectManager::getInstance(QWidget *parent)
{
    if (m_singleton == nullptr)
        m_singleton = new ProjectManager(parent);

    return m_singleton;
}

ProjectManager::ProjectManager(QWidget *parent) : QTreeWidget(parent)
{
    setMinimumWidth(PROJECT_MANAGER_MIN_HEIGHT);
    setMaximumWidth(PROJECT_MANAGER_MAX_HEIGHT);

    setHeaderLabel("Projects");

    m_currentProject = nullptr;
    boldFont.setBold(true);
}

ProjectManager::~ProjectManager()
{
    for (auto p : m_projects)
    {
        saveProjectFile(p);
        p.reset();
        //delete p;
    }
}

bool ProjectManager::newProject(QString path, bool toLoad)
{
    ProjectItem* mainNode(nullptr);

    if (path == "")
        return false;

    // Load project data
    if (toLoad)
    {
        mainNode = openProjectFile(path);

        if (mainNode == nullptr)
            return false;

        // Check files and folders
        if (!checkProjectFiles(mainNode, path))
            return false;
    }
    else // Create default project files
    {
        QFile mainFile(QFileInfo(path).path() + "/Source files/main.has");
        QDir folder;

        folder.mkpath(QFileInfo(path).path() + "/Source files");
        folder.mkpath(QFileInfo(path).path() + "/Libraries");

        if (!mainFile.open(QIODevice::WriteOnly)) // Couldn't create the main file
            return false;
    }

    // Create a new project
    std::shared_ptr<Project> newProject = std::make_shared<Project>(QFileInfo(path).baseName(), path, mainNode);

    m_projects.push_back(newProject);

    addTopLevelItem(newProject->getTopItem());

    if (m_currentProject != nullptr)
        m_currentProject->getTopItem()->setFont(0, defaultFont);

    m_currentProject = newProject;
    newProject->getTopItem()->setFont(0, boldFont);

    // Updating project's tree
    if (!updateProjectFiles(newProject->getTopItem(), newProject->getDirPath()))
        QMessageBox::warning(this, tr("Files or folders missing"), tr("Files or folders of this project are missing.\n\nThey have been removed from the project file."));

    expandAll(newProject->getTopItem());

    // Saving the project file
    saveProjectFile(newProject);

    return true;
}

bool ProjectManager::isOpened(QString path)
{
    for (auto p : m_projects)
    {
        if (p->getPath() == path)
        {
            return true;
        }
    }

    return false;
}

bool ProjectManager::selectProject(QString name)
{
    for (auto p : m_projects)
    {
        if (p->getName() == name)
        {
            m_currentProject->getTopItem()->setFont(0, defaultFont);
            p->getTopItem()->setFont(0, boldFont);

            m_currentProject = p;

            return true;
        }
    }

    return false;
}

void ProjectManager::closeAllProjects()
{
    while (m_projects.count() > 0)
    {
        closeProject(m_projects[0]);
    }
}

bool ProjectManager::closeProject(QString projectName)
{
    for (auto p : m_projects)
    {
        if (p->getName() == projectName)
        {
            return closeProject(p);
        }
    }

    return false;
}

bool ProjectManager::closeProject(std::shared_ptr<Project> project)
{
    bool success(false);

    if (project == nullptr)
        return false;

    saveProjectFile(project);
    success = m_projects.removeOne(project);

    if (project == m_currentProject)
    {
        if (m_projects.count() > 0)
        {
            m_currentProject = m_projects[0];
            m_currentProject->getTopItem()->setFont(0, boldFont);
        }
        else
        {
            m_currentProject.reset();
        }
    }

    project.reset();

    return success;
}

bool ProjectManager::removeCurrentItem()
{
    // Find selected item
    if (selectedItems().isEmpty() || selectedItems().count() > 1)
    {
        return false;
    }

    ProjectItem* selectedItem = dynamic_cast<ProjectItem*>(selectedItems()[0]);

    // If it's a file or a folder (excepted "Source files", "Data" and "Libraries")
    if (selectedItem->isMainNode())
    {
        return false;
    }

    if (selectedItem->getName() == "Source files"
     || selectedItem->getName() == "Libraries"
     || selectedItem->getName() == "main.has")
    {
        return false;
    }

    delete selectedItem;

    return true;
}

ProjectItem::Type ProjectManager::getProjectItemSelectedType()
{
    if (selectedItems().isEmpty() || selectedItems().count() > 1)
    {
        return ProjectItem::Type::None;
    }

    ProjectItem* selectedItem = dynamic_cast<ProjectItem*>(selectedItems()[0]);

    return getProjectItemType(selectedItem);
}

ProjectItem::Type ProjectManager::getProjectItemType(ProjectItem* item)
{
    if (item == nullptr)
        return ProjectItem::Type::None;

    if (item->isMainNode())
        return ProjectItem::Type::ProjectName;
    else if (item->getName() == "Source files" || item->getName() == "Libraries")
    {
        return ProjectItem::Type::MandatoryFolder;
    }
    else if (item->getName() == "main.has")
        return ProjectItem::Type::MandatoryFile;
    else if (item->isFolder())
        return ProjectItem::Type::Folder;
    else
        return ProjectItem::Type::File;
}

std::shared_ptr<Project> ProjectManager::getParentProject(ProjectItem* item)
{
    for (auto p : m_projects)
    {
        if (p->isPartOf(item))
            return p;
    }

    return nullptr;
}

std::shared_ptr<Project> ProjectManager::getCurrentProject()
{
    return m_currentProject;
}

std::shared_ptr<Project> ProjectManager::getProject(QString projectName)
{
    for (auto p : m_projects)
    {
        if (p->getName() == projectName)
            return p;
    }

    return nullptr;
}

ProjectItem* ProjectManager::openProjectFile(QString path)
{
    ProjectItem* mainNode;
    QString projectName;

    QFile xmlFile(path);
    QDomDocument document;

    if (!xmlFile.open(QIODevice::ReadOnly | QIODevice::Text))
    {
        qDebug() << "Failed to open the project file for reading.";
        return nullptr;
    }
    else
    {
        if (!document.setContent(&xmlFile))
        {
            qDebug() << "Failed to load the project file for reading.";
            return nullptr;
        }
    }

    QDomNode root = document.firstChildElement(); // <project name="SOMETHING">

    if (root.nodeName() != "project")
    {
        qDebug() << "Invalid project file";
        return nullptr;
    }

    projectName = root.attributes().namedItem("name").nodeValue();

    if (projectName.isEmpty())
    {
        qDebug() << "Invalid project name in project file";
        return nullptr;
    }

    mainNode = new ProjectItem(projectName);

    for (int i(0); i < root.childNodes().count(); i++)
    {
        if (!readProjectItemXML(root.childNodes().at(i), mainNode))
            return nullptr;
    }

    return mainNode;
}

bool ProjectManager::readProjectItemXML(QDomNode xmlNode, ProjectItem* topProjectNode)
{
    QString itemName;

    if (xmlNode.nodeName() == "folder")
    {
        itemName = xmlNode.attributes().namedItem("name").nodeValue();

        if (itemName.isEmpty())
        {
            qDebug() << "Invalid file name in project file";
            return false;
        }

        ProjectItem* folderItem = topProjectNode->addFolder(itemName);

        for (int i(0); i < xmlNode.childNodes().count(); i++)
        {
            if (!readProjectItemXML(xmlNode.childNodes().at(i), folderItem))
                return false;
        }
    }
    else if (xmlNode.nodeName() == "file")
    {
        itemName = xmlNode.attributes().namedItem("name").nodeValue();

        if (itemName.isEmpty())
        {
            qDebug() << "Invalid file name in project file";
            return false;
        }

        topProjectNode->addFile(itemName);
    }
    else
    {
        qDebug() << "Invalid XML node in project file";
        return false;
    }

    return true;
}

bool ProjectManager::saveProjectFile(std::shared_ptr<Project> p)
{
    QFile xmlFile(p->getPath());

    if (!xmlFile.open(QFile::WriteOnly | QFile::Text))
    {
        qDebug() << "Already opened or there is another issue";
        xmlFile.close();
        return false;
    }

    QTextStream xmlContent(&xmlFile);

    QDomDocument document;

    QDomElement projectRoot = document.createElement("project");
    projectRoot.setAttribute("name", p->getName());
    document.appendChild(projectRoot);

    for (int i(0); i < p->getTopItem()->childCount(); i++)
    {
        ProjectItem* child(dynamic_cast<ProjectItem*>(p->getTopItem()->child(i)));

        projectRoot.appendChild(writeProjectItemXML(child, document));
    }

    xmlContent << document.toString();

    xmlFile.close();

    return true;
}

QDomElement ProjectManager::writeProjectItemXML(ProjectItem* item, QDomDocument &document)
{
    QDomElement itemEl;

    if (item->isFolder())
    {
        itemEl = document.createElement("folder");
        itemEl.setAttribute("name", item->getName());

        for (int i(0); i < item->childCount(); i++)
        {
            ProjectItem* child(dynamic_cast<ProjectItem*>(item->child(i)));

            itemEl.appendChild(writeProjectItemXML(child, document));
        }
    }
    else
    {
        itemEl = document.createElement("file");
        itemEl.setAttribute("name", item->getName());
    }

    return itemEl;
}

bool ProjectManager::checkProjectFiles(ProjectItem* mainNode, QString path)
{
    bool sourceFolderFound(false);
    bool librariesFolderFound(false);
    bool mainFileFound(false);

    for (int i(0); i < mainNode->childCount(); i++)
    {
        ProjectItem *child(dynamic_cast<ProjectItem*>(mainNode->child(i)));

        if (child->isFolder())
        {
            if (child->getName() == "Source files")
            {
                sourceFolderFound = true;

                for (int j(0); j < child->childCount(); j++)
                {
                    ProjectItem* subChild(dynamic_cast<ProjectItem*>(child->child(i)));

                    if (!subChild->isFolder())
                    {
                        if (subChild->getName() == "main.has")
                            mainFileFound = true;
                    }
                }

                if (!mainFileFound)
                    return false;
            }

            if (child->getName() == "Libraries")
                librariesFolderFound = true;
        }
    }

    return sourceFolderFound && librariesFolderFound;
}

bool ProjectManager::updateProjectFiles(ProjectItem* node, QString projectPath)
{
    ProjectItem* child;
    QString path;
    bool allExist(true);

    for (int i(0); i < node->childCount(); i++)
    {
        child = dynamic_cast<ProjectItem*>(node->child(i));
        path = projectPath + child->getPath();

        if (child->isFolder())
        {
            if (QDir(path).exists()) // Folder exists, go deeper
            {
                allExist = updateProjectFiles(child, projectPath);
            }
            else // Folder doesn't exist, remove it from project
            {
                delete child;
                i--;

                allExist = false;
            }
        }
        else
        {
            if (!QFileInfo::exists(path)) // File doesn't exist, remove it from project
            {
                delete child;
                i--;

                allExist = false;
            }
        }
    }

    return allExist;
}

void ProjectManager::expandAll(ProjectItem *item)
{
    expandItem(item);

    for (unsigned int i(0); i < item->childCount(); i++)
    {
        expandAll(dynamic_cast<ProjectItem*>(item->child(i)));
    }
}
