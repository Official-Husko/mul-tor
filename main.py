import inquirer
from termcolor import colored
from time import sleep
import plyer
import os
import json

from modules import *

version = "1.0.0"
files_list = []

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

        # Check if a config exists else create it
        if os.path.exists("config.json"):
            config = Config_Manager.Reader()
        else:
            Config_Manager.Writer()
            config = Config_Manager.Reader()
        
        # Check if the user wants to use proxies and get them
        if config["useProxies"] == True:
            print(colored("Fetching Fresh Proxies...", "yellow"), end='\r')
            proxy_list = ProxyScraper.Scraper()
            print(colored(f"Fetched {len(proxy_list)} Proxies.        ", "green"))
            print("")
        else:
            proxy_list = []
        
        if os.path.exists("user_agents.json"):
            with open("user_agents.json", "r") as ua:
                ua_list = json.load(ua)
                status = UserAgentManager.Verify(ua_list["creation_date"])
                ua.close()
                if status == "OUTDATED":
                    UserAgentManager.Scraper()
        else:
            UserAgentManager.Scraper()
            
        available = Availability_Checker.Evaluate(config)
        
        return config, available, proxy_list, ua_list
        
    def selection(config, available, user_agents_list, proxy_list=""):
        amount_question = [
        inquirer.List('selection',
                            message=colored("What file/s do you want to upload?", "green"),
                            choices=["Single", "Multiple"],
                            ),
        ]
        amount_answers = inquirer.prompt(amount_question)
        print("")
        sites = amount_answers.get("selection")

        
        if amount_answers.get("selection") == "Single":
            files_list = plyer.filechooser.open_file()
        elif amount_answers.get("selection") == "Multiple":
            fn = plyer.filechooser.choose_dir()
            fn = fn[0]
            files_in_folder = os.listdir(fn)
            for found_file in files_in_folder:
                files_list.append(f"{fn}\\{found_file}")
        else:
            print(colored("Something fucked up! Please report this on github. Selecton_Error", "red"))
            sleep(5)
        
        questions = [
        inquirer.Checkbox('selections',
                            message=colored("What sites do you want to upload too?", "green"),
                            choices=available,
                            ),
        ]
        answers = inquirer.prompt(questions)
        print("")

        sites = answers.get("selections")
        
        for site in sites:
            if site == "pixeldrain":
                PixelDrain.Uploader(files_list, proxy_list, user_agents_list)
    
if __name__ == '__main__':
    try:
        startup = Main.startup()
        Main.selection(config=startup[0], available=startup[1], user_agents_list=startup[3]["user_agents"], proxy_list=startup[2])
    except KeyboardInterrupt:
        print("User Cancelled")
        sleep(3)
        exit(0)
        
"""
If you are reading this then beware of wild notes and a rubber duck i let running loose in these lines.

# TODO: Add the PixelDrain List feature
# TODO: Fix runtime.log file being generated in the target files folder alongside file_links.txt
# TODO: Multiply the time and space division by 12 for accurate quantum physics inside of VS Code
 
"""