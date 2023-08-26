#include "projectManager.h"

// === PROJECT NODE CLASS ===
ProjectItem::ProjectItem(QString path, ProjectItem* parent, bool isFolder) : QTreeWidgetItem(parent)
{
    QFileInfo info(path);

    m_isFolder = isFolder;
    m_path = path;
    m_name = m_isFolder ? info.baseName() : info.fileName();

    setText(0, m_name);
}

ProjectItem::ProjectItem(QString projectName) : QTreeWidgetItem()
{
    m_isFolder = true;
    m_path = "";
    m_name = projectName;

    setText(0, projectName);
}

ProjectItem::~ProjectItem()
{}

ProjectItem* ProjectItem::addFolder(QString name)
{
    if (name == "")
        return nullptr;

    QString path(m_path + "/" + name);

    ProjectItem* child = new ProjectItem(path, this, true);
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

    ProjectItem* child = new ProjectItem(path, this, false);
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

bool ProjectItem::setPath(QString newPath)
{
    if (!newPath.isEmpty())
    {
        m_path = newPath;

        return true;
    }

    return false;
}

bool ProjectItem::rename(QString newName, QString currentFullPath)
{
    if (!m_name.isEmpty())
    {
        QFileInfo pathInfo(m_path);
        QFileInfo fullPathInfo(currentFullPath);

        m_name = newName;
        setText(0, m_name);

        QString newPath = pathInfo.absolutePath() + "/" + newName;
        QString newFullPath = fullPathInfo.absolutePath() + "/" + newName;

        QFile::rename(currentFullPath, newFullPath);
        m_path = newPath;

        if (m_isFolder)
            renameInChildrenPaths(newName);

        return true;
    }

    return false;
}

void ProjectItem::renameInChildrenPaths(QString newParentName)
{
    for (unsigned int i(0); i < childCount(); i++)
    {
        ProjectItem *c(dynamic_cast<ProjectItem*>(child(i)));

        c->setPath(c->getPath().replace(1, (c->getPath().indexOf('/', 1) - 1), newParentName));

        if (c->isFolder())
            c->renameInChildrenPaths(newParentName);
    }
}

QString ProjectItem::str(int tab)
{
    QString result("");
    QFileInfo info(m_path);

    if (!isMainNode())
    {
        for (int i(0); i < tab; i++) { result += " "; }
        result += (m_isFolder) ? "Folder " : "File ";
        result += "\"" + info.fileName() + "\" (path: " + m_path + ")\n";
    }

    for (int i(0); i < this->childCount(); i++)
    {
        result += dynamic_cast<ProjectItem*>(this->child(i))->str();
    }

    return result;
}


// === PROJECT CLASS ===
Project::Project(QString name, QString path, ProjectItem* mainNode)
{
    m_name = name;
    m_path = path;

    if (mainNode == nullptr)
    {
        m_topItem = new ProjectItem(name);
        addFolder("Source files");
        addFile("main.has", "/Source files");
        addFolder("Libraries");
    }
    else
        m_topItem = mainNode;
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

QString Project::str()
{
    QString result("");

    result += "\n--- Project " + m_name + " ---\n";
    result += "Path: " + m_path + "\n\n";
    result += m_topItem->str();

    return result;
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
    setMinimumWidth(PROJECT_MAN_MIN_HEIGHT);
    setMaximumWidth(PROJECT_MAN_MAX_HEIGHT);

    setHeaderLabel("Projects");

    m_currentProject = nullptr;
    boldFont.setBold(true);
}

ProjectManager::~ProjectManager()
{
    for (auto p : m_projects)
    {
        saveProjectFile(p);
        delete p;
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
    Project* newProject = new Project(QFileInfo(path).baseName(), path, mainNode);

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

bool ProjectManager::closeProject(Project* p)
{
    if (p == nullptr)
        return false;

    for (int i(0); i < m_projects.count(); i++)
    {
        if (m_projects[i] == p)
        {
            // TODO : Close all editor tabs associated to this project (maybe in mainWindow.cpp, before calling this)
            //        Ensures files to be saved on disk

            saveProjectFile(p);
            delete p;

            m_projects.removeAt(i);

            if (m_projects.count() > 0)
            {
                m_currentProject = m_projects[0];
                m_currentProject->getTopItem()->setFont(0, boldFont);
            }
            else
                m_currentProject = nullptr;

            return true;
        }
    }

    return false;
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

ItemType ProjectManager::getItemSelectType()
{
    if (selectedItems().isEmpty() || selectedItems().count() > 1)
    {
        return ItemType::None;
    }

    ProjectItem* selectedItem = dynamic_cast<ProjectItem*>(selectedItems()[0]);

    return getItemType(selectedItem);
}

ItemType ProjectManager::getItemType(ProjectItem* item)
{
    if (item == nullptr)
        return ItemType::None;

    if (item->isMainNode())
        return ItemType::ProjectName;
    else if (item->getName() == "Source files" || item->getName() == "Libraries")
    {
        return ItemType::MandatoryFolder;
    }
    else if (item->getName() == "main.has")
        return ItemType::MandatoryFile;
    else if (item->isFolder())
        return ItemType::Folder;
    else
        return ItemType::File;
}

Project* ProjectManager::getParentProject(ProjectItem* item)
{
    for (auto p : m_projects)
    {
        if (p->isPartOf(item))
            return p;
    }

    return nullptr;
}

QString ProjectManager::getFirstProjectName()
{
    if (m_projects.count() > 0)
        return m_projects[0]->getName();

    return "";
}

Project* ProjectManager::getCurrentProject()
{
    return m_currentProject;
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

bool ProjectManager::saveProjectFile(Project* p)
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
