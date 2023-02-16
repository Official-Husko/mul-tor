import requests
import random
import base64
from termcolor import colored
import inquirer
import webbrowser
import os

from main import version
from .logger import Logger
from .pretty_print import error

public_tokens = [
    "Z2hwX2l3bzZPcDV1dUxSQXNTTm92WHdETDROaWJXOW91cDNCYW1sWg==",
]

class AutoUpdate:
    
    def Checker(proxy_list, ua_list):
        try:
            apiKey = base64.b64decode(random.choice(public_tokens)).decode('utf-8')
            ua = random.choice(ua_list)
            url = "https://api.github.com/repos/Official-Husko/mul-tor/releases/latest"
            
            headers = {
                "User-Agent": ua,
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
                "Authorization": f"Bearer {apiKey}"
            }
            
            if proxy_list == []:
                req = requests.get(url, headers=headers).json()
            else:
                proxy = random.choice(proxy_list)
                req = requests.get(url, headers=headers, proxies=proxy).json()
                
                
            repo_version = req.get("tag_name").split("-")
            repo_version_n = repo_version[0].replace("v", "")
            try:
                repo_version_type = repo_version[1]
            except:
                pass
            
            if ("pre", "dev", "exp", "de") in repo_version:
                return
            
            if str(version) <= repo_version_n:
                print(colored("UPDATE AVAILABLE!", "red", attrs=["blink"]))
                
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
                    webbrowser.open("https://github.com/Official-Husko/mul-tor/releases/latest", new=2)
                    os.exit(0)
        
        except Exception as e:
            # Construct and print the error
            error_str = f"An error occured while checking for updates! Please report this. Exception: {e}"
            print(f"{error} {error_str}")
            Logger.log_event(error_str, req)