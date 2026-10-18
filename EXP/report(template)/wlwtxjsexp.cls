\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{wlwtxjsexp}[2026/10/18 WLWTXJS experiment report]

\DeclareOption*{\PassOptionsToClass{\CurrentOption}{cqutreport}}
\ProcessOptions\relax
\LoadClass{cqutreport}

\RequirePackage[
  a4paper,
  top=1.35cm,
  bottom=1.35cm,
  left=2.5cm,
  right=2.5cm
]{geometry}
\RequirePackage{array}

\renewcommand{\arraystretch}{1.0}
\setlength{\fboxsep}{8pt}
\setlength{\fboxrule}{0.4pt}

\newcommand{\coverfield}[2]{%
  \makebox[2.0cm][s]{#1} &
  \underline{\makebox[7.3cm][c]{#2}}\\[0.56cm]
}

\newcommand{\reportbox}[3]{%
  \noindent\fbox{%
    \begin{minipage}[t][#2][t]{\dimexpr\linewidth-2\fboxsep-2\fboxrule\relax}
      \noindent\textbf{#1}\par
      \vspace{0.18cm}
      #3
    \end{minipage}%
  }\par
  \nointerlineskip
  \vspace{-\fboxrule}%
}

\newcommand{\makecover}{%
  \begin{titlepage}
    \thispagestyle{empty}
    \centering
    \vspace*{1.15cm}
    \IfFileExists{assets/cqut-logo-horizontal.png}{%
      \cqutCoverLogoHorizontal[11.2cm]%
    }{{\heiti\fontsize{28pt}{34pt}\selectfont 重庆理工大学}}\par
    \vspace{2.1cm}
    {\heiti\zihao{2} GPIO 输出控制实验-流水灯\par}
    \vspace{1.85cm}
    {\zihao{4}
      \begin{tabular}{>{\raggedleft\arraybackslash}p{2.0cm}@{\hspace{0.45cm}}p{7.3cm}}
        \coverfield{学\hfill 院}{计算机科学与工程学院}
        \coverfield{学\hfill 号}{}
        \coverfield{姓\hfill 名}{}
        \coverfield{指导教师}{}
        \coverfield{使用教材}{}
        \coverfield{编写时间}{}
      \end{tabular}
    }
    \vfill
  \end{titlepage}%
}

\let\cqut@savedBeginReportBody\cqutBeginReportBody
\renewcommand{\cqutBeginReportBody}{%
  \thispagestyle{empty}%
  \cqut@savedBeginReportBody
  \zihao{5}%
}

\endinput
