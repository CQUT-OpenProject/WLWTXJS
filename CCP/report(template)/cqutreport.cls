%% Copyright 2026 CQUT-OpenProject
%%
%% This work may be distributed and/or modified under the
%% conditions of the LaTeX Project Public License, version 1.3c.
%%
%% This work has the LPPL maintenance status `maintained'.
%% The Current Maintainer of this work is CQUT-OpenProject.
%%
\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{cqutreport}[2026/10/18 v1.0 CQUT course report base]

\DeclareOption*{\PassOptionsToClass{\CurrentOption}{ctexart}}
\ProcessOptions\relax
\LoadClass[UTF8,a4paper,zihao=-4,fontset=none,oneside]{ctexart}

\RequirePackage{cqutreportfonts}
\cqutApplySchoolFonts

\endinput
