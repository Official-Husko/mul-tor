# Import Standard Libraries
import os

# This file simply holds some data that is used throughout the project.

__version__: str = "2.0.0"
__user_agent__: str = f"mul-tor/{__version__}- (by Official Husko on GitHub)"
__dev_debug__: bool = os.path.exists(".DEBUG")
