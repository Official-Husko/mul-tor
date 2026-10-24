---
name: Bug report
about: Something does not work as it should
title: ''
labels: bug
assignees: Official-Husko

---

**What happened?**
A clear and concise description of the problem, and what you expected instead.

**How to reproduce it**
The exact command you ran (`mul-tor upload ... --provider ...`), or the steps you took in the graphical version.

**Which program and version?**
Output of `mul-tor --version` (or `mul-tor-gui --version`), and whether you use the command line or the graphical version.

**Operating system**
e.g. Windows 11, macOS 15, Ubuntu 24.04.

**Provider(s) affected**
If a particular host fails, name it, and say whether it worked before.

**The log**
The relevant lines from the log (`mul-tor logs -n 100`, or the files in the folder `mul-tor logs --path` prints; run again with `-vv` for more detail). It contains the provider's reply, which usually shows what went wrong.
**Remove any API keys, passwords or private links first.**

**Anything else**
Screenshots, proxy settings, anything unusual about your setup.
