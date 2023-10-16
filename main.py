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

DEBUG = False

version = "1.2.0"
owd = os.getcwd()
platform = sys.platform

if platform == "win32":
    from ctypes import windll

if os.path.exists("outdated"):
    version_for_logo = colored(f"v{version}", "cyan", attrs=["blink"])
else:
    version_for_logo = colored(f"v{version}", "cyan")

logo = f"""{colored(f'''
.88b  d88. db    db db             d888888b  .d88b.  d8888b. 
88'YbdP`88 88    88 88             `~~88~~' .8P  Y8. 88  `8D 
88  88  88 88    88 88                88    88    88 88oobY' 
88  88  88 88    88 88      C8888D    88    88    88 88`8b   
88  88  88 88b  d88 88booo.           88    `8b  d8' 88 `88. 
YP  YP  YP ~Y8888P' Y88888P           YP     `Y88P'  88   YD 
                                    {version_for_logo} | by {colored("Official-Husko", "yellow")}''', "red")}
"""
class Main:
    
    def startup():
        if platform == "win32":
            os.system("cls")
            windll.kernel32.SetConsoleTitleW(f"Mul-Tor | v{version}")
        print(logo)

        if DEBUG == True:
            print(f"{colored('Platform:', 'green')} {platform}")
            print("")

        # Check if a config exists else create it
        if os.path.exists("config.json"):
            config = Config_Manager.Reader()
        else:
            Config_Manager.Writer()
            config = Config_Manager.Reader()
        
        proxies_enabled = config.get("useProxies", False)
        random_ua_enabled = config.get("randomUserAgent", False)
        check_for_updates_enabled = config.get("checkForUpdates", False)
        
        if check_for_updates_enabled == True:
            os.system("cls")
            print(logo)
            print("")
            print(colored("Checking for Updates...", "yellow"), end='\r')
            AutoUpdate.Checker()
            os.system("cls")
            print(logo)
            print("")

        # Check if the user wants to use proxies and get them
        if proxies_enabled == True:
            print(colored("Fetching Fresh Proxies...", "yellow"), end='\r')
            proxy_list = ProxyScraper.Scraper()
            print(f"{colored(f'Fetched', 'green')} {colored(len(proxy_list), 'yellow')} {colored('Proxies.        ', 'green')}")
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
        if DEBUG == True:
            print(available)


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
                print(colored("Something fucked up! Please report this on github. Selection_Error", "red"))
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
                    bar_file_name = os.path.basename(file)
                    list_bar.title = f'-> Uploading {colored(bar_file_name, "light_blue")} to {colored(site, "yellow")}, please wait...'
                    output = {
                        "status": "",
                        "file_site": "",
                        "file_name": "",
                        "file_url": "",
                        "exception": "",
                        "size_limit": "",
                        "extra": ""
                    }

                    uploader_classes = {
                        "PixelDrain": PixelDrain,
                        "GoFile": GoFile,
                        "Oshi": Oshi,
                        "FileBin": FileBin,
                        "Delafil": Delafil,
                        "Files.dp.ua": Files_dp_ua,
                        "FilesFM": FilesFM,
                        "Krakenfiles": Krakenfiles,
                        "Transfer": Transfer,
                        "TmpFiles": TmpFiles,
                        "Mixdrop": Mixdrop,
                        "1Fichier": OneFichier,
                        "YourFileStore": YourFileStore,
                        "Fileio": Fileio,
                        "EasyUpload": EasyUpload
                    }

                    if site in uploader_classes:
                        output = uploader_classes.get(site, "No_Site").Uploader(file, proxy_list, user_agents_list)
                    else:
                        pass

                    # All sites below this line are in beta state. Expect them to have mental breakdowns. 

                    status = output.get("status", "404 status not found")
                    file_site = output.get("site", "well fuck me there was no site")
                    file_name = output.get("file_name", "oopsie_daisie.fish")
                    file_url = output.get("file_url", "url be doing the hidy hole")
                    exception_str = output.get("exception", "Fuck me there was no exception.")
                    size_limit = output.get("size_limit", "-3 GB")
                    extra = output.get("extra", "Monkey stole the bananas")
                    
                    os.chdir(owd)
                    if status == "ok":
                        print(f"{ok} {colored(file_name, 'light_blue')} {colored('successfully uploaded to', 'green')} {colored(file_site, 'yellow')}{colored('! URL:', 'green')} {colored(file_url, 'light_blue')}")
                        with open("file_links.txt", "a") as file_links:
                            file_links.writelines(f"{datetime.now()} | {site} | {file_name} - {file_url}\n")
                        file_links.close()
                    
                    elif status == "error":
                        print(f"{error} An error occurred while uploading the file {colored(file_name, 'light_blue')} to {colored(file_site, 'yellow')}! Please report this. Exception: {colored(exception_str, 'red')}")
                        error_str = f"An error occurred while uploading the file {file_name} to {file_site}! Please report this. Exception: {exception_str}"
                        Logger.log_event(error_str, extra)
                        
                    elif status == "size_error":
                        print(f"{error} File size of {colored(file_name, 'light_blue')} to big for {colored(file_site, 'yellow')}! Compress it to fit the max size of {colored(size_limit, 'yellow')}")
                        error_str = f"File size of {file_name} to big for {file_site}! Compress it to fit the max size of {size_limit}"
                        Logger.log_event(error_str, extra)    
                    
                    else:
                        print(f"{major_error} An unknown error occurred while uploading the file {colored(file_name, 'light_blue')} to {colored(file_site, 'yellow')}! Please report this. Exception: {colored(exception_str, 'red')}")
                        error_str = f"An unknown error occurred while uploading the file {file_name} to {file_site}! Please report this. Exception: {exception_str}"
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

# TODO: Might be just me but this code is dog shit
# TODO: Multiply time and space by 12 then divide by 25 for accurate quantum physics inside of VS Codium
# TODO: add a working progress bar to each upload. Possible solution https://stackoverflow.com/questions/13909900/progress-of-python-requests-post
# TODO: Find a way to change the colors for the selection windows
# TODO: Finish this so i can start learning Rust *Turns out im too retarded for Rust*. Im learning Godot instead
# TODO: Since September 12th 2023 and even before that to be honest i've had a special note to unity. GO FUCK YOURSELF.
# TODO: Fix all broken sites since Anonfiles simulated The Soviet union in 1991
# TODO: Add presets system
# TODO: Quack
# TODO: Simplify Code. I think this is possible and should be done in order to maintain a clean and easy to read code. This should also make it easier to maintain
"""

"""
Here you can also find some buried credits.

Arrow Icon from Kirill Kazachek on https://flaticon.com/authors/kirill-kazachek
Wolf Icon from Iconriver on https://flaticon.com/authors/iconriver
Combined App Icon from my slippery hotdog fingers on https://Its3AMIReallyShouldGoToBed.edu.uk.xyz.dev/http_code_418

"""