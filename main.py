import requests
import inquirer
from termcolor import colored
from time import sleep
import plyer
import subprocess
import os
import json

from modules import *

version = "1.0.0"
available = []
target_files = []

logo = f"""{colored(f'''
.88b  d88. db    db db             d888888b  .d88b.  d8888b. 
88'YbdP`88 88    88 88             `~~88~~' .8P  Y8. 88  `8D 
88  88  88 88    88 88                88    88    88 88oobY' 
88  88  88 88    88 88      C8888D    88    88    88 88`8b   
88  88  88 88b  d88 88booo.           88    `8b  d8' 88 `88. 
YP  YP  YP ~Y8888P' Y88888P           YP     `Y88P'  88   YD 
                                    {colored(version,"cyan")} | by {colored("Official-Husko", "yellow")}''', "red")}
"""
class Main:
    
    def startup():
        print(logo)
        # TODO: Fetch Proxies
        
        if os.path.exists("user_agents.json"):
            with open("user_agents.json", "r") as ua:
                ua_list = json.load(ua)
                status = UserAgentManager.Verify(ua_list["creation_date"])
                ua.close()
                if status == "OUTDATED":
                    UserAgentManager.Scraper()
                    print(status)
        else:
            UserAgentManager.Scraper()
            
        if os.path.exists("config.json"):
            with open("config.json", "r") as cf:
                pass
        else:
            with open("config.json") as cf:
                pass
        
    def selection():
        amount_question = [
        inquirer.List('selection',
                            message=colored("What file or files do you want to upload?", "green"),
                            choices=["Single", "Multiple"],
                            ),
        ]
        amount_answers = inquirer.prompt(amount_question)
        print("")
        sites = amount_answers.get("selection")

        
        if amount_answers.get("selection") == "Single":
            fn = plyer.filechooser.open_file()
            target_files.append(fn[0])
        elif amount_answers.get("selection") == "Multiple":
            fn = plyer.filechooser.choose_dir()
            fn = fn[0]
            files_in_folder = os.listdir(fn)
            for found_file in files_in_folder:
                target_files.append(f"{fn}\\{found_file}")
        else:
            print(colored("Something fucked up! Please report this on github. Selecton_Error", "red"))
            sleep(5)
        print(fn)
        print(target_files)
        questions = [
        inquirer.Checkbox('selections',
                            message=colored("What sites do you want to upload too?", "green"),
                            choices=available,
                            ),
        ]
        answers = inquirer.prompt(questions)
        print("")

        sites = answers.get("selections")
        print(sites)
        
        for site in sites:
            if site == "pixeldrain":
                PixelDrain.UPLOADER()
    
if __name__ == '__main__':
    try:
        Main.startup()
        Main.selection()
    except KeyboardInterrupt:
        print("User Cancelled")
        sleep(3)
        exit(0)