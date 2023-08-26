
#ifndef PROJECTMANAGER_H
#define PROJECTMANAGER_H

#include <QVector>
#include <QList>
#include <QFileInfo>
#include <QTreeWidget>
#include <QXmlStreamWriter>
#include <QXmlStreamReader>
#include <QDomElement>
#include <QFile>
#include <QDir>
#include <QMessageBox>

#define PROJECT_MAN_MIN_HEIGHT 200
#define PROJECT_MAN_MAX_HEIGHT 300

enum class ItemType { ProjectName, File, Folder, MandatoryFile, MandatoryFolder, None };

class ProjectItem : public QTreeWidgetItem
{
    public:
        ProjectItem(QString path, ProjectItem* parent, bool isFolder);
        ProjectItem(QString projectName);
        ~ProjectItem();

        ProjectItem* addFolder(QString name); // Both return new child pointer, nullptr if error
        ProjectItem* addFile(QString name);

        QString getName();
        QString getPath();
        QList<QString> getFilesNamesList();
        bool isFolder();
        bool isMainNode();
        bool isPartOf(ProjectItem* item);
        QList<QString> getFilesPaths(QString projectPath = "");

        bool setPath(QString newPath);

        bool rename(QString newName, QString currentFullPath);
        void renameInChildrenPaths(QString newParentName);

        QString str(int tab = -1);

    private:
        bool m_isFolder;
        QString m_path;
        QString m_name;
};

class Project
{
    public:
        Project(QString name, QString path, ProjectItem* mainNode = nullptr);
        ~Project();

        bool addFolder(QString name, QString path = "/");
        bool addFile(QString name, QString path);

        static QList<QString> getPathParentsList(QString path);

        QString getName();
        QString getDirPath();
        QString getPath();
        bool getAssembled();
        ProjectItem* getTopItem();
        bool isPartOf(ProjectItem* item);
        QList<QString> getFilesPaths();
        QList<QString> getFilesNames();

        void setAssembled(bool assembled);

        QString str();

    private:
        QString m_name;
        QString m_path;
        bool m_assembled;

        ProjectItem *m_topItem;
};

class ProjectManager : public QTreeWidget // SINGLETON
{
    Q_OBJECT

        static ProjectManager *m_singleton;

    public:
        static ProjectManager* getInstance(QWidget *parent);

        ~ProjectManager();

        bool newProject(QString path, bool toLoad = false);
        bool selectProject(QString name);
        void closeAllProjects();
        bool closeProject(QString projectName);
        bool closeProject(Project* p);
        bool removeCurrentItem();
        ItemType getItemSelectType();
        ItemType getItemType(ProjectItem* item);
        Project* getParentProject(ProjectItem* item);
        Project* getCurrentProject();

    private:
        ProjectManager(QWidget *parent);

        ProjectItem* openProjectFile(QString path); // Returns project mainNode (nullptr if error)
        bool readProjectItemXML(QDomNode xmlNode, ProjectItem* topProjectNode);
        bool saveProjectFile(Project* p);
        QDomElement writeProjectItemXML(ProjectItem* item, QDomDocument &document);

        bool checkProjectFiles(ProjectItem* mainNode, QString path);
        bool updateProjectFiles(ProjectItem* node, QString path); // Removes files and folders from project if they don't exist

        void expandAll(ProjectItem *item);

        QVector<Project*> m_projects;
        Project* m_currentProject;

        QFont defaultFont;
        QFont boldFont;
};

#endif // PROJECTMANAGER_H
