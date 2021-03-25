//! Collection of conversions from `pennsieve-rust` crate models
//! to cli printable representations.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::From;
use std::fmt::{self, Display};

use prettytable::{self as pt, cell, row};

use pennsieve_rust::api::response;
use pennsieve_rust::model;

use crate::ps::agent::cli;
use crate::ps::agent::database::{UploadRecords, UserRecord};
use crate::ps::util::temporal::timespec_to_rfc3339;

// ~~~ ApiSettings ~~~
#[derive(Debug, Clone)]
pub struct CliSettings(HashMap<String, String>);

impl CliSettings {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for CliSettings {
    type Item = (String, String);
    type IntoIter = ::std::collections::hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<HashMap<String, String>> for CliSettings {
    fn from(settings: HashMap<String, String>) -> Self {
        CliSettings(settings)
    }
}

impl Display for CliSettings {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(Some(vec!["KEY", "VALUE"]), move |t| {
            let mut entries: Vec<(String, String)> = self.clone().into_iter().collect::<Vec<_>>();
            entries.sort_by(|(ka, _), (kb, _)| ka.cmp(kb));
            entries.iter().for_each(|(k, v)| {
                t.add_row(row![k, v]);
            });
        })
        .fmt(fmt)
    }
}

// ~~~ Collaborators ~~~
pub struct CliCollaborators {
    pub organizations: CliOrganizationRoles,
    pub teams: CliTeams,
    pub users: CliUsers,
}

impl Display for CliCollaborators {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let organizations = &self.organizations;
        let organizations = if organizations.is_empty() {
            cell!("none")
        } else {
            cell!(cli::table(Some(vec!["NAME", "ROLE"]), move |org_table| {
                organizations
                    .clone()
                    .into_iter()
                    .for_each(|o: response::OrganizationRole| {
                        org_table.add_row(row![
                            Into::<String>::into(o.name().clone()),
                            Into::<String>::into(
                                o.role().cloned().unwrap_or_else(|| "none".to_string())
                            ),
                        ]);
                    });
            }))
        };

        let teams = &self.teams;
        let teams = if teams.is_empty() {
            cell!("none")
        } else {
            cell!(cli::table(Some(vec!["NAME", "ROLE"]), move |team_table| {
                teams.clone().into_iter().for_each(|team: CliTeam| {
                    team_table.add_row(row![
                        Into::<String>::into(team.0.name().clone()),
                        Into::<String>::into(
                            team.0.role().cloned().unwrap_or_else(|| "none".to_string())
                        ),
                    ]);
                });
            }))
        };

        let users = if self.users.is_empty() {
            cell!("none")
        } else {
            cell!(self.users.minimal_table_with_roles())
        };

        cli::table(Some(vec!["ORGANIZATIONS", "TEAMS", "USERS"]), move |t| {
            t.add_row(row![organizations, teams, users,]);
        })
        .fmt(fmt)
    }
}

// ~~~ Organizations ~~~

#[derive(Debug, Clone)]
pub struct CliOrganizations(response::Organizations);

impl CliOrganizations {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for CliOrganizations {
    type Item = response::Organization;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<response::Organizations> for CliOrganizations {
    fn from(organizations: response::Organizations) -> Self {
        CliOrganizations(organizations)
    }
}

impl From<Vec<model::Organization>> for CliOrganizations {
    fn from(organizations: Vec<model::Organization>) -> Self {
        Into::<response::Organizations>::into(organizations).into()
    }
}

impl Display for CliOrganizations {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len() > 0 {
            cli::table(Some(vec!["ORGANIZATION"]), move |t| {
                self.clone()
                    .into_iter()
                    .for_each(|o: response::Organization| {
                        t.add_row(row![Into::<String>::into(o.organization().name().clone())]);
                    });
            })
            .fmt(fmt)
        } else {
            writeln!(fmt, "No organizations")
        }
    }
}

// ~~~ OrganizationRoles ~~~

#[derive(Debug, Clone)]
pub struct CliOrganizationRoles(Vec<response::OrganizationRole>);

impl CliOrganizationRoles {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for CliOrganizationRoles {
    type Item = response::OrganizationRole;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<response::OrganizationRole>> for CliOrganizationRoles {
    fn from(organizations: Vec<response::OrganizationRole>) -> Self {
        CliOrganizationRoles(organizations)
    }
}

impl From<response::OrganizationRole> for CliOrganizationRoles {
    fn from(organization: response::OrganizationRole) -> Self {
        CliOrganizationRoles(vec![organization])
    }
}

impl Display for CliOrganizationRoles {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_empty() {
            cli::table(Some(vec!["ORGANIZATION", "ROLE"]), move |t| {
                self.clone()
                    .into_iter()
                    .for_each(|o: response::OrganizationRole| {
                        t.add_row(row![
                            Into::<String>::into(o.name().clone()),
                            Into::<String>::into(
                                o.role().cloned().unwrap_or_else(|| "none".to_string())
                            )
                        ]);
                    });
            })
            .fmt(fmt)
        } else {
            writeln!(fmt, "No organizations")
        }
    }
}

// ~~~ Packages ~~~

#[derive(Debug, Clone)]
pub struct CliPackages(Vec<CliPackage>);

impl CliPackages {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<CliPackage>> for CliPackages {
    fn from(packages: Vec<CliPackage>) -> Self {
        CliPackages(packages)
    }
}

impl IntoIterator for CliPackages {
    type Item = CliPackage;
    type IntoIter = ::std::vec::IntoIter<CliPackage>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Display for CliPackages {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.len() > 0 {
            cli::table(Some(vec!["NAME", "ID"]), move |t| {
                self.clone().into_iter().for_each(|p: CliPackage| {
                    t.add_row(row![
                        p.content.name(),
                        Into::<String>::into(p.content.id().clone()),
                    ]);
                });
            })
            .fmt(fmt)
        } else {
            writeln!(fmt, "No packages")
        }
    }
}

// ~~~ Package ~~~

#[derive(Debug, Clone)]
pub struct CliPackage {
    children: Vec<CliPackage>,
    content: model::Package,
}

impl CliPackage {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.children.len()
    }
}

impl IntoIterator for CliPackage {
    type Item = CliPackage;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}

impl From<response::Package> for CliPackage {
    fn from(package: response::Package) -> Self {
        let mut children = package
            .children()
            .unwrap_or(&vec![])
            .iter()
            .cloned()
            .map(Into::<CliPackage>::into)
            .collect::<Vec<_>>();
        children.sort_by(|a, b| {
            a.content
                .name()
                .to_lowercase()
                .cmp(&b.content.name().to_lowercase())
        });
        Self {
            children,
            content: package.take(),
        }
    }
}

impl Display for CliPackage {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(Some(vec!["NAME", "ID", "DATASET ID"]), |t| {
            t.add_row(row![
                self.content.name(),
                Into::<String>::into(self.content.id().clone()),
                Into::<String>::into(self.content.dataset_id().clone())
            ]);
        })
        .fmt(fmt)
    }
}

// ~~~ Collection ~~~

/// Collections consist of a root package and its children:
#[derive(Debug, Clone)]
pub struct CliCollection(CliPackage);

impl CliCollection {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<CliPackage> for CliCollection {
    fn from(root: CliPackage) -> Self {
        CliCollection(root)
    }
}

impl From<response::Package> for CliCollection {
    fn from(collection: response::Package) -> Self {
        CliCollection(collection.into())
    }
}

impl IntoIterator for CliCollection {
    type Item = CliPackage;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Display for CliCollection {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(fmt)?;
        if self.len() == 0 {
            return write!(fmt, "Empty collection");
        }
        cli::table(Some(vec!["NAME", "ID"]), move |t| {
            self.clone().into_iter().for_each(|p| {
                t.add_row(row![
                    p.content.name(),
                    Into::<String>::into(p.content.id().clone()),
                ]);
            });
        })
        .fmt(fmt)
    }
}

// ~~~ Dataset ~~~

#[derive(Debug, Clone)]
pub struct CliDataset {
    children: CliPackages,
    content: model::Dataset,
}

impl CliDataset {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.children.len()
    }
}

impl From<response::Dataset> for CliDataset {
    fn from(dataset: response::Dataset) -> Self {
        let mut children = dataset
            .children()
            .unwrap_or(&vec![])
            .iter()
            .cloned()
            .map(Into::<CliPackage>::into)
            .collect::<Vec<_>>();
        children.sort_by(|a, b| a.content.name().cmp(b.content.name()));
        Self {
            children: CliPackages(children),
            content: dataset.take(),
        }
    }
}

impl Display for CliDataset {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(Some(vec!["NAME", "DESCRIPTION", "STATUS", "ID"]), |t| {
            t.add_row(row![
                self.content.name(),
                self.content.description().unwrap_or(&"".to_owned()),
                self.content.status().to_owned(),
                Into::<String>::into(self.content.id().clone()),
            ]);
        })
        .fmt(fmt)?;
        if self.children.len() > 0 {
            writeln!(fmt)?;
            self.children.fmt(fmt)?;
        }
        Ok(())
    }
}

// ~~~ Datasets ~~~

#[derive(Debug, Clone)]
pub struct CliDatasets(Vec<CliDataset>);

impl CliDatasets {
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for CliDatasets {
    type Item = CliDataset;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        // Sort before returning
        let mut ds = self.0;
        ds.sort_by(|a, b| {
            a.content
                .name()
                .to_lowercase()
                .cmp(&b.content.name().to_lowercase())
        });
        ds.into_iter()
    }
}

impl From<Vec<CliDataset>> for CliDatasets {
    fn from(dataset: Vec<CliDataset>) -> Self {
        CliDatasets(dataset)
    }
}

impl Display for CliDatasets {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(Some(vec!["DATASET", "NAME", "STATUS"]), |t| {
            for r in self.clone() {
                t.add_row(row![
                    pt::Cell::new(r.content.id().as_ref()),
                    cell!(r.content.name()),
                    cell!(r.content.status().to_owned())
                ]);
            }
        })
        .fmt(fmt)
    }
}

// ~~~ User ~~~

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliUser(model::User);

impl From<model::User> for CliUser {
    fn from(user: model::User) -> Self {
        CliUser(user)
    }
}

impl Ord for CliUser {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.last_name().cmp(&other.0.last_name())
    }
}

impl PartialOrd for CliUser {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.last_name().cmp(&other.0.last_name()))
    }
}

impl Display for CliUser {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(
            Some(vec!["LAST NAME", "FIRST NAME", "EMAIL", "USER ID"]),
            |t| {
                t.add_row(row![
                    cell!(self.0.last_name()),
                    cell!(self.0.first_name()),
                    cell!(self.0.email()),
                    pt::Cell::new(self.0.id().borrow()),
                ]);
            },
        )
        .fmt(fmt)
    }
}

// ~~~ Users ~~~

#[derive(Debug, Clone)]
pub struct CliUsers(Vec<model::User>);

impl CliUsers {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    fn table<F>(&self, columns: Vec<&str>, create_row: F) -> cli::CliTable
    where
        F: Fn(model::User) -> prettytable::Row,
    {
        cli::table(Some(columns), |t| {
            let mut users = self.0.clone();
            users.sort_by(|a, b| a.last_name().cmp(&b.last_name()));
            for r in users {
                t.add_row(create_row(r));
            }
        })
    }

    pub fn minimal_table_with_roles(&self) -> cli::CliTable {
        self.table(vec!["LAST NAME", "FIRST NAME", "EMAIL", "ROLE"], |r| {
            row![
                cell!(r.last_name()),
                cell!(r.first_name()),
                cell!(r.email()),
                cell!(r.role().cloned().unwrap_or_else(|| "none".to_string())),
            ]
        })
    }

    pub fn table_with_roles(&self) -> cli::CliTable {
        self.table(
            vec!["LAST NAME", "FIRST NAME", "EMAIL", "ROLE", "ID"],
            |r| {
                row![
                    cell!(r.last_name()),
                    cell!(r.first_name()),
                    cell!(r.email()),
                    cell!(r.role().cloned().unwrap_or_else(|| "none".to_string())),
                    pt::Cell::new(r.id().borrow()),
                ]
            },
        )
    }

    pub fn table_without_roles(&self) -> cli::CliTable {
        self.table(vec!["LAST NAME", "FIRST NAME", "EMAIL", "ID"], |r| {
            row![
                cell!(r.last_name()),
                cell!(r.first_name()),
                cell!(r.email()),
                pt::Cell::new(r.id().borrow()),
            ]
        })
    }
}

impl From<Vec<model::User>> for CliUsers {
    fn from(users: Vec<model::User>) -> Self {
        CliUsers(users)
    }
}

impl IntoIterator for CliUsers {
    type Item = CliUser;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let users = self
            .0
            .into_iter()
            .map(Into::<CliUser>::into)
            .collect::<Vec<_>>();
        users.into_iter()
    }
}

// ~~~ Teams ~~~

#[derive(Debug, Clone)]
pub struct CliTeams(Vec<model::Team>);

impl CliTeams {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl From<Vec<model::Team>> for CliTeams {
    fn from(teams: Vec<model::Team>) -> Self {
        CliTeams(teams)
    }
}

impl From<Vec<CliTeam>> for CliTeams {
    fn from(teams: Vec<CliTeam>) -> Self {
        let teams = teams.into_iter().map(|t| t.0).collect();
        CliTeams(teams)
    }
}

impl IntoIterator for CliTeams {
    type Item = CliTeam;
    type IntoIter = ::std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let users = self
            .0
            .into_iter()
            .map(Into::<CliTeam>::into)
            .collect::<Vec<_>>();
        users.into_iter()
    }
}

impl Display for CliTeams {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(Some(vec!["NAME", "ROLE", "ID"]), |t| {
            let mut teams = self.0.clone();
            teams.sort_by(|a, b| a.name().cmp(&b.name()));
            for r in teams {
                t.add_row(row![
                    cell!(r.name()),
                    cell!(r.role().cloned().unwrap_or_else(|| "none".to_string())),
                    pt::Cell::new(r.id().as_ref()),
                ]);
            }
        })
        .fmt(fmt)
    }
}

// ~~~ Team ~~~

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliTeam(model::Team);

impl From<model::Team> for CliTeam {
    fn from(user: model::Team) -> Self {
        CliTeam(user)
    }
}

impl Ord for CliTeam {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.name().cmp(&other.0.name())
    }
}

impl PartialOrd for CliTeam {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.0.name().cmp(&other.0.name()))
    }
}

impl Display for CliTeam {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(Some(vec!["NAME", "ID"]), |t| {
            t.add_row(row![
                cell!(self.0.name()),
                pt::Cell::new(self.0.id().as_ref()),
            ]);
        })
        .fmt(fmt)
    }
}

// ~~~ UserRecord ~~~

impl Display for UserRecord {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(None as Option<Vec<&str>>, |t| {
            t.add_row(row!["NAME", self.name]);
            t.add_row(row!["USER ID", self.id]);
            t.add_row(row!["ORGANIZATION", self.organization_name]);
            t.add_row(row!["ORGANIZATION ID", self.organization_id]);
        })
        .fmt(fmt)
    }
}

// ~~~ UploadRecords ~~~

pub struct CliUploadRecords(UploadRecords);

impl From<UploadRecords> for CliUploadRecords {
    fn from(records: UploadRecords) -> Self {
        CliUploadRecords(records)
    }
}

impl Display for CliUploadRecords {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        cli::table(
            Some(vec![
                "ID",
                "FILE",
                "CREATED AT",
                "DATASET",
                "PACKAGE",
                "STATUS",
                "APPEND",
                "% DONE",
            ]),
            |t| {
                for r in &self.0.records {
                    t.add_row(row![
                        pt::Cell::new(
                            r.id.map(|id| id.to_string())
                                .unwrap_or_else(|| "N/A".to_string())
                                .as_ref(),
                        ),
                        pt::Cell::new(r.file_path.as_ref()),
                        pt::Cell::new(timespec_to_rfc3339(r.created_at).as_ref()),
                        pt::Cell::new(r.dataset_id.as_ref()),
                        pt::Cell::new(
                            r.package_id
                                .clone()
                                .unwrap_or_else(|| "N/A".to_string())
                                .as_ref(),
                        ),
                        pt::Cell::new(r.status.as_ref()),
                        pt::Cell::new(if r.append { "true" } else { "false" }),
                        pt::Cell::new(r.progress.to_string().as_ref()),
                    ]);
                }
            },
        )
        .fmt(fmt)
    }
}
