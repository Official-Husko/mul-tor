import inquirer
from termcolor import colored
from time import sleep
import plyer
import os
import json
from alive_progress import alive_bar
import sys
from datetime import datetime

from modules import *

version = "1.1.0"
owd = os.getcwd()
platform = sys.platform

if platform == "win32":
    from ctypes import windll


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
        if platform == "win32":
            os.system("cls")
            windll.kernel32.SetConsoleTitleW(f"Mul-Tor | v{version}")
        print(logo)

        # Check if a config exists else create it
        if os.path.exists("config.json"):
            config = Config_Manager.Reader()
        else:
            Config_Manager.Writer()
            config = Config_Manager.Reader()
        
        proxies_enabled = config.get("useProxies", False)
        random_ua_enabled = config.get("randomUserAgent", False)
        check_for_updates_enabled = config.get("checkForUpdates", False)
        
        # Check if the user wants to use proxies and get them
        if proxies_enabled == True:
            print(colored("Fetching Fresh Proxies...", "yellow"), end='\r')
            proxy_list = ProxyScraper.Scraper()
            print(colored(f"Fetched {len(proxy_list)} Proxies.        ", "green"))
            print("")
        else:
            proxy_list = []
        
        if random_ua_enabled == True:
            if os.path.exists("user_agents.json"):
                ua_list = UserAgentManager.Reader()
            else:
                ua_list = UserAgentManager.Scraper()
        else:
            ua_list = [f"mul-tor/{version} (by Official Husko on GitHub)"]
            
        available = Availability_Checker.Evaluate(config, proxy_list, ua_list)
        
        if check_for_updates_enabled == True:
            AutoUpdate.Checker(proxy_list, ua_list)
        
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

        files_list = []
        while len(files_list) == 0 or files_list == [[]]:
            files_list = [] # Reset it
            if amount_answers.get("selection") == "Single":
                files_list = plyer.filechooser.open_file()
            elif amount_answers.get("selection") == "Multiple":
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
        
        with alive_bar(len(files_list), calibrate=1, dual_line=True, title='Uploading', enrich_print=False, stats=False, receipt=False, receipt_text=False) as list_bar:
            for file in files_list:
                for site in sites:
                    bar_file_name = file.rsplit("\\")
                    list_bar.title = f'-> Uploading {colored(bar_file_name[-1], "light_blue")} to {colored(site, "yellow")}, please wait...'
                    if site == "PixelDrain":
                        output = PixelDrain.Uploader(file, proxy_list, user_agents_list)
                    if site == "GoFile":
                        output = GoFile.Uploader(file, proxy_list, user_agents_list)
                    if site == "AnonFiles":
                        output = AnonFiles.Uploader(file, proxy_list, user_agents_list)
                    if site == "BayFiles":
                        output = BayFiles.Uploader(file, proxy_list, user_agents_list)
                    if site == "OpenLoad":
                        output = OpenLoad.Uploader(file, proxy_list, user_agents_list)
                    if site == "HotFile":
                        output = HotFile.Uploader(file, proxy_list, user_agents_list)
                    if site == "LolaBits":
                        output = LolaBits.Uploader(file, proxy_list, user_agents_list)
                    if site == "RapidShare":
                        output = RapidShare.Uploader(file, proxy_list, user_agents_list)
                    if site == "UpVid":
                        output = UpVid.Uploader(file, proxy_list, user_agents_list)
                    if site == "vShare":
                        output = vShare.Uploader(file, proxy_list, user_agents_list)
                    if site == "LetsUpload":
                        output = LetsUpload.Uploader(file, proxy_list, user_agents_list)
                    if site == "ShareOnline":
                        output = ShareOnline.Uploader(file, proxy_list, user_agents_list)
                    if site == "MegaUpload":
                        output = MegaUpload.Uploader(file, proxy_list, user_agents_list)
                    if site == "MyFile":
                        output = MyFile.Uploader(file, proxy_list, user_agents_list)
                    if site == "FileChan":
                        output = FileChan.Uploader(file, proxy_list, user_agents_list)
                    if site == "Oshi":
                        output = Oshi.Uploader(file, proxy_list, user_agents_list)
                        
                    if site == "MixDrop":
                        output = MixDrop.Uploader(file, proxy_list, user_agents_list, config)
                        
                    status = output.get("status", "")
                    file_site = output.get("site", "")
                    file_name = output.get("file_name", "")
                    file_url = output.get("file_url", "")
                    exception_str = output.get("exception", "Fuck me there was no exception.")
                    size_limit = output.get("size_limit", "")
                    extra = output.get("extra", "")
                    
                    os.chdir(owd)
                    if status == "ok":
                        print(f"{ok} {colored(file_name, 'light_blue')} successfully uploaded! URL: {colored(file_url, 'green')}")
                        with open("file_links.txt", "a") as file_links:
                            file_links.writelines(f"{datetime.now()} | {site} | {file_name} - {file_url}\n")
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
                list_bar()      
            os.chdir(owd) # reset to default working dir

if __name__ == '__main__':
    try:
        startup = Main.startup()
        while True:
            Main.selection(config=startup[0], available=startup[1], user_agents_list=startup[3], proxy_list=startup[2])
            print("")
            print("")
    except KeyboardInterrupt:
        print("User Cancelled")
        sleep(3)
        exit(0)
        
"""
If you are reading this then beware of wild notes and a rubber duck i let running loose in these lines.

# TODO: Add the PixelDrain List feature
# TODO: Multiply time and space by 12 then divide by 25 for accurate quantum physics inside of VS Code
# TODO: add a working progress bar to each upload. Possible solution https://stackoverflow.com/questions/13909900/progress-of-python-requests-post
# TODO: Find a way to change the colors for the selection windows
# TODO: Finish this so i can start learning Rust
 
"""

"""
Here you can also find some buried credits.

Arrow Icon from Kirill Kazachek on https://www.flaticon.com/authors/kirill-kazachek
Wolf Icon from Iconriver on https://www.flaticon.com/authors/iconriver

"""