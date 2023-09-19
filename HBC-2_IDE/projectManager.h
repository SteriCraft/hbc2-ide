#ifndef PROJECTMANAGER_H
#define PROJECTMANAGER_H

/*!
 * \file projectManager.h
 * \brief The class that stores and manages opened projects
 * \author Gianni Leclercq
 * \version 0.1
 * \date 27/08/2023
 */
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

#include "config.h"

/*!
 * \brief Custom dialog to create a new project
 *
 * Allows to specify multiple settings for the new project (included libraries, interrupt handler, etc.)
 */
class NewProjectDialog : public QDialog
{
    Q_OBJECT

    public:
        /*!
         * \brief Create a custom new project dialog
         */
        NewProjectDialog(bool &ok, QString &projectPath,
                         bool &includeInterruptHandler, bool &includeMathLibrary,
                         ConfigManager *configManager, QWidget *parent = nullptr);

    protected slots:
        void browse(); //!< Allows to browse for a directory for the new project
        void checkNameAndDirectoryValidity(); //!< Checks if either the project name or directory is empty
        void accept(); //!< Overriden slot from QDialog
        void reject(); //!< Overriden slot from QDialog

    private:
        bool &m_ok;
        QString &m_projectPath;
        bool &m_includeInterruptHandler;
        bool &m_includeMathLibrary;

        QLineEdit *m_projectNameLineEdit;
        QLineEdit *m_projectDirectoryLineEdit;

        QCheckBox *m_createSubdirectoryCheckBox;

        QCheckBox *m_includeInterruptHandlerCheckBox;
        QCheckBox *m_includeMathLibraryCheckBox;

        QPushButton *m_createButton;
        QPushButton *m_cancelButton;
};

/*!
 * \brief Describes a custom project item in the project tree
 *
 * Derived from QTreeWidgetItem
 */
class ProjectItem : public QTreeWidgetItem
{
    public:
        enum class Type { ProjectName, File, Folder, MandatoryFile, MandatoryFolder, None }; //!< Lists item types

        /*!
         * \brief Creates an child item
         * \param path Path to the file or folder represented
         * \param parent Pointer to parent item <b>mandatory</b>
         * \param isFolder <b>true</b> if it is a directory
         */
        ProjectItem(QString path, QString associatedProjectName, ProjectItem *parent, bool isFolder);

        ProjectItem(QString projectName); //!< Creates a project's main item
        ~ProjectItem();

        /*!
         * \brief Adds a new child folder
         *
         * Returns <b>nullptr</b> if an error occured
         *
         * \return a pointer to the created item
         */
        ProjectItem* addFolder(QString name); // Both return new child pointer, nullptr if error

        /*!
         * \brief Adds a new child file
         *
         * Returns <b>nullptr</b> if an error occured
         *
         * \return a pointer to the created item
         */
        ProjectItem* addFile(QString name);

        QString getName();
        QString getPath();
        QList<QString> getFilesNamesList(); //!< Returns list of children file's names <b>(recursive)</b>
        bool isFolder();
        bool isMainNode();

        /*!
         * \brief Returns if the passed item is one of the children
         *
         * <i>Used by Project::isPartOf() for recursive check.</i>
         *
         * \param item
         * \return
         */
        bool isPartOf(ProjectItem* item); //!< Returns <b>true</b> if the passed item is a child of this item

        /*!
         * \brief Returns a list of children file's path
         *
         * \param projectPath The project's path, which is added to each file path in output
         */
        QList<QString> getFilesPaths(QString projectPath = "");

        /*!
         * \brief Recursive
         * \return a list of pointers to ProjectItems that are children files
         */
        QList<ProjectItem*> getFilesItems();

        /*!
         * \brief Returns the ProjectItem associated to the given path
         *
         * Includes children items
         */
        ProjectItem* getProjectItemFromPath(QString fullPath, QString projectPath);

        QString getAssociatedProjectName();

        bool setPath(QString newPath); //!< Returns <b>false</b> if newPath is empty

        void setAssociatedProjectName(QString associatedProjectName);

        /*!
         * \brief Renames the item
         * \return <b>false</b> if the new name is empty
         */
        bool rename(QString newName, QString currentFullPath);

        /*!
         * \brief Renames items recursively
         *
         * Called if a folder has been renamed. See ProjectItem::rename.
         */
        void renameParentInChildrenPaths(QString oldPartialParentPath, QString newPartialParentPath);

        /*!
         * \return <b>true</b> if the item or any of its child has that path
         */
        bool contains(QString path, QString projectPath);

    private:
        bool m_isFolder;
        QString m_path;
        QString m_name;
        QString m_associatedProjectName;
};

/*!
 * \brief Describes a project
 */
class Project
{
    public:
        /*!
         * \brief Creates a new project
         * \param name Project name
         * \param path Project directory
         * \param mainNode Complete tree of an opened project <i><b>(leave empty if it is a new project)</b></i>
         */
        Project(QString name, QString path, ProjectItem* mainNode = nullptr);
        ~Project();

        /*!
         * \brief Adds a folder to the project
         * \param name Folder name
         * \param path Folder path in the project <b>(not an absolute path)</b>
         * \return <b>false</b> if one of the parent directories wasn't found <b>(invalid path)</b>
         */
        bool addFolder(QString name, QString path = "/");

        /*!
         * \brief Adds a file to the project
         * \param name File name
         * \param path File path in the project <b>(not an absolute path)</b>
         * \return <b>false</b> if one of the parent directories wasn't found <b>(invalid path)</b>
         */
        bool addFile(QString name, QString path);

        /*!
         * \return a pointer to the ProjectItem associated to the given full path
         * \return <b>nullptr</b> if there is no such ProjectItem
         */
        ProjectItem* getProjectItemFromFullPath(QString fullPath);

        static QList<QString> getPathParentsList(QString path); //!< Converts a path in a list of directory (main one first)

        QString getName();
        QString getDirPath();
        QString getPath();
        bool getAssembled(); //!< Returns if the project is currently assembled in binary
        ProjectItem* getTopItem(); //!< Returns the tree's main node
        bool isPartOf(ProjectItem* item); //!< Returns <b>true</b> if the passed item is a child of this project <b>(recursive)</b>
        QList<QString> getFilesPaths(); //!< Returns a list of the files complete paths
        QList<QString> getFilesNames(); //!< Returns a list of the files names

        /*!
         * \return <b>true</b> if the project already includes a file or folder with that path
         */
        bool contains(QString path);

        void setAssembled(bool assembled);

    private:
        QString m_name;
        QString m_path;
        bool m_assembled;

        ProjectItem *m_topItem;
};

/*!
 * \class ProjectManager
 * \brief Singleton of the project manager
 *
 * Creates and manages projects
 */
class ProjectManager : public QTreeWidget // SINGLETON
{
    Q_OBJECT

    static ProjectManager *m_singleton;

    public:
        static ProjectManager* getInstance(QWidget *parent); //!< <i><b>SINGLETON:</b></i> Call this to instanciate the object (the constructor is private).
        ~ProjectManager();

        /*!
         * \brief Creates a new project
         * \param path Project's main directory
         * \param toLoad <b>true</b> if the project must be loaded <i>(open a project)</i>
         * \return <b>false</b> if path is empty
         * \return <b>false</b> if the project could not be loaded
         * \return <b>false</b> if the project's files are invalid
         * \return <b>false</b> if the new project could not be created
         */
        bool newProject(QString path, bool toLoad = false);

        /*!
         * \return <b>true</b> if the project is already opened
         */
        bool isOpened(QString path);

        /*!
         * \brief Changes the currently active project
         * \return <b>false</b> if the project could not be found
         */
        bool selectProject(QString name);
        void closeAllProjects();

        /*!
         * \brief Closes a project
         * \return <b>false</b> if the project could not be found
         */
        bool closeProject(QString projectName);

        /*!
         * \brief Closes a project
         * \return <b>false</b> if the project could not be found or if p is <b>nullptr</b>
         */
        bool closeProject(std::shared_ptr<Project> project);

        /*!
         * \brief Removes the currently selected item
         *
         * "Source files" directory, "Libraries" directory and "main.has" source file are mandatory items.
         *
         * \return <b>false</b> if there is no selected item
         * \return <b>false</b> if the selected item is a project's main node
         * \return <b>false</b> if the selected item is a mandatory item
         */
        bool removeCurrentItem();

        /*!
         * \brief Returns selected item type (<b>ProjectItem::Type::None</b> if none)
         */
        ProjectItem::Type getProjectItemSelectedType();

        /*!
         * \brief Returns item type (<b>ProjectItem::Type::None</b> if no item passed)
         */
        ProjectItem::Type getProjectItemType(ProjectItem* item);

        /*!
         * \brief Returns a pointer to the parent Project of the item
         * \return <b>nullptr</b> if the item is part of no project
         */
        std::shared_ptr<Project> getParentProject(ProjectItem* item);

        /*!
         * \brief Returns currently active project (<b>nullptr</b> if none)
         */
        std::shared_ptr<Project> getCurrentProject();

        /*!
         * \brief Returns a shared pointer to the project corresponding to the given name
         * \return <b>nullptr</b> if there is no such project
         */
        std::shared_ptr<Project> getProject(QString projectName);

    private:
        ProjectManager(QWidget *parent);

        ProjectItem* openProjectFile(QString path); // Returns project mainNode (nullptr if error)
        bool readProjectItemXML(QDomNode xmlNode, ProjectItem* topProjectNode);
        bool saveProjectFile(std::shared_ptr<Project> p);
        QDomElement writeProjectItemXML(ProjectItem* item, QDomDocument &document);

        bool checkProjectFiles(ProjectItem* mainNode, QString path);
        bool updateProjectFiles(ProjectItem* node, QString path); // Removes files and folders from project if they don't exist

        void expandAll(ProjectItem *item);

        static constexpr int PROJECT_MANAGER_MIN_HEIGHT = 200;
        static constexpr int PROJECT_MANAGER_MAX_HEIGHT = 300;

        QVector<std::shared_ptr<Project>> m_projects;
        std::shared_ptr<Project> m_currentProject;

        QFont defaultFont;
        QFont boldFont;
};

#endif // PROJECTMANAGER_H
