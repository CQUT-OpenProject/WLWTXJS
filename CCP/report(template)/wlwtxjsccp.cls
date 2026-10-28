\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{wlwtxjsccp}[2026/10/28 WLWTXJS communication protocol design report]

\DeclareOption*{\PassOptionsToClass{\CurrentOption}{cqutreport}}
\ProcessOptions\relax
\LoadClass{cqutreport}

\RequirePackage[
  a4paper,
  top=2.54cm,
  bottom=2.54cm,
  left=3.18cm,
  right=3.18cm,
  headheight=15pt,
  headsep=0.45cm,
  footskip=0.72cm
]{geometry}
\RequirePackage{xcolor}
\RequirePackage{array}
\RequirePackage{tabularx}
\RequirePackage{makecell}
\RequirePackage{fancyhdr}
\RequirePackage{titletoc}
\RequirePackage{listings}
\RequirePackage{tikz}
\usetikzlibrary{arrows.meta}
\RequirePackage{hyperref}

\hypersetup{hidelinks,plainpages=false}
\renewcommand{\arraystretch}{1.0}

\newcolumntype{C}[1]{>{\centering\arraybackslash}p{#1}}
\newcolumntype{Y}{>{\raggedright\arraybackslash}X}
\newcolumntype{Z}{>{\centering\arraybackslash}X}

% 章标题：第一章 / 节标题：1.1 / 小节标题：1.1.1
\ctexset{
  contentsname = {目\quad 录},
  section = {
    name = {,},
    number = {第\chinese{section}章},
    format = \centering\heiti\bfseries\zihao{3},
    beforeskip = 0pt,
    afterskip = 18pt
  },
  subsection = {
    number = {\arabic{section}.\arabic{subsection}},
    format = \heiti\bfseries\zihao{4},
    beforeskip = 14pt,
    afterskip = 8pt
  },
  subsubsection = {
    number = {\arabic{section}.\arabic{subsection}.\arabic{subsubsection}},
    format = \heiti\bfseries\zihao{-4},
    beforeskip = 10pt,
    afterskip = 6pt
  }
}

% 每章另起一页（含目录、摘要、附录）
\let\cqut@origsection\section
\renewcommand{\section}{\clearpage\cqut@origsection}

\setcounter{tocdepth}{2}
\titlecontents{section}[0pt]
  {\setstretch{1.2}\heiti\bfseries}
  {\contentspush{\thecontentslabel\enspace}}
  {}
  {\titlerule*[0.5pc]{.}\contentspage}
\titlecontents{subsection}[2em]
  {\setstretch{1.2}}
  {\contentspush{\thecontentslabel\enspace}}
  {}
  {\titlerule*[0.5pc]{.}\contentspage}

\fancypagestyle{reporthead}{
  \fancyhf{}
  \fancyhead[C]{\zihao{-5}重庆理工大学物联网通信技术设计报告}
  \renewcommand{\headrulewidth}{0.4pt}
  \renewcommand{\footrulewidth}{0pt}
}

\fancypagestyle{reportfront}{
  \fancyhf{}
  \fancyhead[C]{\zihao{-5}重庆理工大学物联网通信技术设计报告}
  \fancyfoot[C]{\zihao{-5}\thepage}
  \renewcommand{\headrulewidth}{0.4pt}
  \renewcommand{\footrulewidth}{0pt}
}

\fancypagestyle{reportbody}{
  \fancyhf{}
  \fancyhead[C]{\zihao{-5}重庆理工大学综合课程设计报告}
  \fancyfoot[C]{\zihao{-5}\thepage}
  \renewcommand{\headrulewidth}{0.4pt}
  \renewcommand{\footrulewidth}{0pt}
}

\newcommand{\tabhead}[1]{{\heiti\bfseries #1}}
\newcommand{\fld}[2]{\makecell{#1\\(#2)}}
\newcommand{\devtosrv}{设备\ensuremath{\rightarrow}服务器}
\newcommand{\srvtodev}{服务器\ensuremath{\rightarrow}设备}

\newenvironment{reptable}{%
  \begin{center}%
  \zihao{5}\setstretch{1}%
  \renewcommand{\arraystretch}{1.6}%
}{%
  \end{center}%
}

\newcommand{\leaditem}[2]{\noindent\textbf{#1：}#2\par}
\newcommand{\subleaditem}[2]{{\setlength{\leftskip}{2em}\noindent\textbf{#1：}#2\par}}
\newcommand{\blocklabel}[1]{\noindent\textbf{#1}\par}

\newcounter{appitem}
\newcommand{\appitem}[1]{%
  \stepcounter{appitem}%
  \subsubsection*{\arabic{appitem}\enspace #1}%
}

\newcommand{\unnumberedsection}[1]{%
  \clearpage
  \phantomsection
  \addcontentsline{toc}{section}{#1}%
  \section*{#1}%
}

\newenvironment{reportabstract}[1]{%
  \unnumberedsection{摘\quad 要}%
  \def\cqut@keywords{#1}%
}{%
  \par\noindent\textbf{关键词：}\cqut@keywords\par
}

\lstdefinestyle{appendixcode}{
  basicstyle=\ttfamily\zihao{5},
  breaklines=true,
  columns=fullflexible,
  keepspaces=true,
  showstringspaces=false,
  tabsize=4,
  xleftmargin=2em,
  aboveskip=0.5em,
  belowskip=0.5em
}
\lstset{style=appendixcode}

% 时序图与流程图：左为设备侧（生命线 x=2.9cm），右为服务器侧（x=8.9cm）
\tikzset{
  seqhead/.style={draw, rounded corners=2pt, minimum width=2.6cm, minimum height=0.7cm, inner sep=3pt},
  seqline/.style={densely dashed, line width=0.4pt},
  seqact/.style={draw, rounded corners=2pt, fill=white, inner sep=3pt, align=center},
  seqarrow/.style={-{Latex[length=2mm]}, line width=0.5pt},
  seqnote/.style={anchor=west, text width=5.4cm, inner sep=1pt, align=left}
}
\newenvironment{seqdiagram}{%
  \begin{center}\setstretch{1}%
  \begin{tikzpicture}[font=\small]%
}{%
  \end{tikzpicture}\end{center}%
}
\newcommand{\seqtitles}[2]{%
  \node[seqhead] at (2.9,0) {#1};
  \node[seqhead] at (8.9,0) {#2};
}
\newcommand{\seqlines}[1]{%
  \draw[seqline] (2.9,-0.4) -- (2.9,-#1);
  \draw[seqline] (8.9,-0.4) -- (8.9,-#1);
}
\newcommand{\seqact}[2]{\node[seqact] at (2.9,-#1) {#2};}
\newcommand{\seqsend}[2]{\draw[seqarrow] (2.9,-#1) -- (8.9,-#1) node[midway, above=1pt] {#2};}
\newcommand{\seqrecv}[2]{\draw[seqarrow] (8.9,-#1) -- (2.9,-#1) node[midway, above=1pt] {#2};}
\newcommand{\seqnote}[2]{\node[seqnote] at (9.1,-#1) {#2};}

\newcommand{\coverrow}[2]{%
  \noindent\makebox[2.4cm][s]{#1}\hspace{0.3cm}%
  \underline{\makebox[9.0cm][c]{#2}}\par\vspace{0.45cm}%
}
\newcommand{\coverline}{%
  \noindent\underline{\makebox[11.7cm]{}}\par\vspace{0.45cm}%
}

\newcommand{\makecover}{%
  \thispagestyle{reporthead}%
  \begingroup
  \setstretch{1}%
  \centering
  \vspace*{0.6cm}%
  \cqutCoverLogoHorizontal[11.2cm]\par
  \vspace{1.0cm}%
  {\heiti\bfseries\zihao{1} 物联网通信技术设计报告\par}
  \vspace{1.2cm}%
  \zihao{4}%
  \begin{minipage}{11.7cm}
    \coverrow{二级学院}{计算机科学与工程学院}
    \coverrow{专\hfill 业}{物联网工程}
    \coverrow{班\hfill 级}{}
    \noindent\makebox[2.4cm][s]{学生姓名}\hspace{0.3cm}%
    \underline{\makebox[3.6cm][c]{}}\hspace{0.3cm}学号\hspace{0.3cm}%
    \underline{\makebox[3.8cm][c]{}}\par\vspace{0.45cm}%
    \coverrow{题\hfill 目}{校园热水站数据采集通信协议设计报告}
    \coverrow{评\hfill 语}{}
    \coverline
    \coverline
  \end{minipage}\par
  \vfill
  {\zihao{5}\renewcommand{\arraystretch}{1.5}\setlength{\tabcolsep}{3pt}%
  \begin{tabular}{|C{3.1cm}|C{4.6cm}|C{2.8cm}|C{3.1cm}|}
    \hline
    \tabhead{系统设计（50\%）} & \tabhead{课堂答辩及报告（50\%）} & \tabhead{总评（100\%）} & \tabhead{总评教师}\\
    \hline
    \rule{0pt}{1.6cm} & & & \\
    \hline
  \end{tabular}}\par
  \endgroup
  \clearpage
  \pagestyle{reportfront}%
  \pagenumbering{Roman}%
  \cqut@savedBeginReportBody
}

% 正文：宋体小四、约 1.5 倍行距
\let\cqut@savedBeginReportBody\cqutBeginReportBody
\renewcommand{\cqutBeginReportBody}{%
  \clearpage
  \pagestyle{reportbody}%
  \pagenumbering{arabic}%
  \cqut@savedBeginReportBody
}

\endinput
