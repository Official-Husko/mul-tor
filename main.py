import inquirer
from termcolor import colored
from time import sleep
import plyer
import os
import json

from modules import *

version = "1.0.0"
owd = os.getcwd()


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
        
        if config["randomUserAgent"] == True:
            if os.path.exists("user_agents.json"):
                with open("user_agents.json", "r") as ua:
                    ua_list = json.load(ua)
                    status = UserAgentManager.Verify(ua_list["creation_date"])
                    ua.close()
                    if status == "OUTDATED":
                        UserAgentManager.Scraper()
            else:
                UserAgentManager.Scraper()
        else:
            ua = {
                "creation_date": f"1970-01-01",
                "user_agents": [f"mul-tor/{version} (by Official Husko on GitHub)"]
            }
            
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
            files_list = []
            fn = plyer.filechooser.choose_dir()
            fn = fn[0]
            files_in_folder = os.listdir(fn)
            for found_file in files_in_folder:
                if os.path.isdir(f"{fn}\\{found_file}") != True:
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
        
        for file in files_list:
            for site in sites:
                if site == "PixelDrain":
                    output = PixelDrain.Uploader(file, proxy_list, user_agents_list)
                if site == "GoFile":
                    output = GoFile.Uploader(file, proxy_list, user_agents_list)
                
                status = output.get("status", "")
                file_site = output.get("site", "")
                file_name = output.get("file_name", "")
                file_url = output.get("file_url", "")
                exception_str = output.get("exception", "")
                size_limit = output.get("size_limit", "")
                extra = output.get("extra", "")
                
                os.chdir(owd)
                if status == "ok":
                    print(f"{ok} {colored(file_name, 'light_blue')} successfully uploaded! URL: {colored(file_url, 'green')}")
                    with open("file_links.txt", "a") as file_links:
                        file_links.writelines(f"{site} | {file_name} - {file_url}\n")
                    file_links.close()
                
                elif status == "error":
                    print(f"{error} An error occured while uploading the file {colored(file_name, 'light_blue')} to {colored(file_site, 'yellow')}! Please report this. Exception: {colored(exception_str, 'red')}")
                    error_str = f"An error occured while uploading the file {file_name} to {file_site}! Please report this. Exception: {exception_str}"
                    Logger.log_event(error_str, extra)
                    
                elif status == "size_error":
                    print(f"{error} File size of {colored(file_name, 'light_blue')} to big for {colored(file_site, 'yellow')}! Compress it to fit the max size of {colored(size_limit, 'yellow')}")
                    error_str = f"File size of {file_name} to big for {file_site}! Compress it to fit the max size of {size_limit}"
                    Logger.log_event(error_str, extra)    
                
                else:
                    print(f"{major_error} An unknown error occured while uploading the file {colored(file_name, 'light_blue')} to {colored(file_site, 'yellow')}! Please report this. Exception: {colored(exception_str, 'red')}")
                    error_str = f"An unknown error occured while uploading the file {file_name} to {file_site}! Please report this. Exception: {exception_str}"
                    Logger.log_event(error_str, extra)
                
        os.chdir(owd) # reset to default working dir
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
# TODO: Multiply time and space by 12 then divide by 25 for accurate quantum physics inside of VS Code
 
"""