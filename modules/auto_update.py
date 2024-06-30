# Import Standard Libraries
import os
from time import sleep

# Import Third-Party Libraries
import requests
from termcolor import colored
import inquirer
from alive_progress import alive_bar

# Import Local Libraries
from .storage import __version__ as VERSION
from .storage import __user_agent__ as USER_AGENT
from .logger import Logger


class AutoUpdateChecker:

    def __init__(self) -> None:
        """
        Initializes the AutoUpdateChecker class.

        This constructor sets the initial values for the repository URL, user agent, and tool version.
        The repository URL is set to the GitHub API endpoint for the latest releases of the mul-tor repository.
        The user agent is set to the value of the USER_AGENT constant from the storage module.
        The tool version is set to the value of the VERSION constant from the storage module.

        Parameters:
            None

        Returns:
            None
        """

        self.repository_url: str = "https://api.github.com/repos/Official-Husko/mul-tor/releases/latest"
        self.user_agent: str = USER_AGENT
        self.tool_version: str = VERSION

    def _fetch_repo_details(self) -> dict:
        headers = {
            "User-Agent": self.user_agent, 
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28"
        }
        
        response = requests.get(self.repository_url, headers=headers, timeout=15)

        if not response.status_code == 200:
            raise Exception(response.status_code)

        return response

    def _extract_repo_details(self, request_response: dict):

        return request_response.get("tag_name", "0.0.1")

    def _checker(self):
        try:
            
            headers = {
                "User-Agent":f"mul-tor/{VERSION} (by Official Husko on GitHub)", 
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28"
            }
            
            req = requests.get(self.repository_url, headers=headers).json()
            repo_version = req.get("tag_name").replace("v", "")
            download_link = req["assets"][0]["browser_download_url"]
            
            if str(VERSION) < repo_version:
                print(colored("UPDATE AVAILABLE!      ", "red", attrs=["blink"]))
                
                body = req.get("body")
                name = req.get("name")
                date = req.get("published_at").replace("T", " ").replace("Z", "")
                
                print("")
                print(f"Latest release is {colored(name, 'light_blue')} released on {colored(date, 'yellow')}")
                print("")
                print(body)
                print("")
                amount_question = [
                inquirer.List('selection',
                                    message=colored("Do you want to download the update?", "green"),
                                    choices=["Yes", "No"],
                                    ),
                ]
                amount_answers = inquirer.prompt(amount_question)
                print("")
                decision = amount_answers.get("selection")
                if decision == "Yes":
                    r = requests.get(download_link, headers={"User-Agent":f"mul-tor/{VERSION} (by Official Husko on GitHub)"}, timeout=60, stream=True)
                    with alive_bar(int(int(r.headers.get('content-length')) / 1024 + 1)) as progress_bar:
                        progress_bar.text = f'-> Downloading Update {repo_version}, please wait...'
                        file = open(f"mul-tor-{repo_version}.exe", 'wb')
                        for chunk in r.iter_content(chunk_size=1024):
                            if chunk:
                                file.write(chunk)
                                file.flush()
                                progress_bar()
                    print(f"{ok} Update successfully downloaded! The program will now close and delete the old exe.")
                    if os.path.exists("delete-exe.bat"):
                        os.remove("delete-exe.bat")
                    with open("delete-exe.bat", "a", encoding="utf-8") as bat_creator:
                        bat_content = f'TASKKILL -F /IM Mul-Tor.exe\ntimeout 3\nDEL .\\Mul-Tor.exe\nren .\\mul-tor-{repo_version}.exe Mul-Tor.exe\nDEL .\\delete-exe.bat'
                        bat_creator.write(bat_content)
                        bat_creator.close()
                    os.startfile(r".\\delete-exe.bat")
                    sleep(5)
                    exit(0)
                elif decision == "No":
                    if not os.path.exists("outdated"):
                        with open("outdated", "a", encoding="utf-8") as mark_outdated:
                            mark_outdated.close()
            elif str(VERSION) >= repo_version:
                try:
                    os.remove("outdated")
                except Exception:
                    pass
        
        except Exception as e:
            # Construct and print the error
            error_str = f"An error occurred while checking for updates! Please report this. Exception: {e}"
            print(f"{error} {error_str}")
            Logger.log_event(error_str, req)
            sleep(7)