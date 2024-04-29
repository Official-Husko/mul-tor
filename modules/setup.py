# Import Basics
from time import sleep
import os
import sys
from datetime import datetime

# Import Third Party
import inquirer
from termcolor import colored
import plyer
from alive_progress import alive_bar

# Import custom modules
from .config_manager import Config_Manager
from .auto_update import AutoUpdate
from .proxy_scraper import ProxyScraper
from .logger import Logger
from .pretty_print import error, ok, major_error
from .availability_checker import AvailabilityChecker


# Check if the files exist
if os.path.exists(".DEBUG"):
    DEBUG = True


class Main:

    def __init__(self):
        self.version = "2.0.0"
        self.owd = os.getcwd()
        self.platform = sys.platform
        self.user_agent = f"mul-tor/{self.version}- (by Official Husko on GitHub)" 
        self.version_for_logo = colored(f"v{self.version}", "cyan")
        self.config = None

        self.logo = f"""
            {colored(f'''
            .88b  d88. db    db db             d888888b  .d88b.  d8888b. 
            88'YbdP`88 88    88 88             `~~88~~' .8P  Y8. 88  `8D 
            88  88  88 88    88 88                88    88    88 88oobY' 
            88  88  88 88    88 88      C8888D    88    88    88 88`8b   
            88  88  88 88b  d88 88booo.           88    `8b  d8' 88 `88. 
            YP  YP  YP ~Y8888P' Y88888P           YP     `Y88P'  88   YD 
                                                {self.version_for_logo} | by {colored("Official-Husko", "yellow")}''', "red")}
        """
    
    def startup(self):
        if self.platform == "win32":
            os.system("cls")
        print(self.logo)

        if DEBUG:
            print(f"{colored('Platform:', 'green')} {self.platform}")
            print("")

        # Check if a config exists else create it
        config = Config_Manager.Checker()
        
        proxies_enabled = config.get("useProxies", False)
        check_for_updates_enabled = config.get("checkForUpdates", True)
        
        if check_for_updates_enabled:
            os.system("cls")
            print(self.logo)
            print("")
            print(colored("Checking for Updates...", "yellow"), end='\r')
            AutoUpdate.Checker()
            os.system("cls")
            print(self.logo)
            print("")

        # Check if the user wants to use proxies and get them
        if proxies_enabled:
            print(colored("Fetching Fresh Proxies...", "yellow"), end='\r')
            proxy_list = ProxyScraper.Scraper()
            print(f"{colored('Fetched', 'green')} {colored(len(proxy_list), 'yellow')} {colored('Proxies.        ', 'green')}")
            print("")
        else:
            proxy_list = []

        available = AvailabilityChecker.Evaluate(config, proxy_list)
        if DEBUG:
            print(available)

        if not os.path.exists("presets"):
            os.mkdir("presets")
        if not os.path.exists("presets/readme.txt"):
            with open("presets\\readme.txt", "a", encoding="utf-8") as readme:
                text = "To create your own preset visit the wiki here: https://github.com/Official-Husko/mul-tor/wiki/Preset-Configuration"
                readme.write(text)

        return config, available, proxy_list, ua_list
        
    def selection(config, available, user_agents_list, proxy_list=""):

        auto_load_preset = config.get("presetSystem", {}).get("autoLoadPreset", False)
        enable_preset_selection = config.get("presetSystem", {}).get("enablePresetSelection", False)
        preset_name = config.get("presetSystem", {}).get("presetName", "")

        if auto_load_preset == True and not os.path.exists(f"presets/{preset_name}"):
            print(colored(f"Error: Preset {preset_name} does not exist. Continuing without preset!", "red"))
            print("")
            auto_load_preset = False

        if auto_load_preset == True and DEBUG == False:
            auto_load_data = Preset_Manager.loader(available, preset_name)
            available = auto_load_data[0]
            link_format = auto_load_data[1]
        else:
            link_format = ""

        if DEBUG == True and use_test_file == True:
            if test_small_file == True:
                files_list = [f"{owd}\\test.png"]
            elif test_large_file == True:
                files_list = [f"{owd}\\big_game.zip"]
            elif test_very_large_file == True:
                files_list = [f"{owd}\\very_big_game.7z"]
            else:
                print(colored("Something fucked up! Please report this on github. Test_File_Error", "red"))

        else:
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
        
        if available == []:
            print(colored("No sites are available. Please double check your config (and preset if used). If you think this is an error please report it on github.", "red"))
            sleep(10)
            exit(0)

        questions = [
        inquirer.Checkbox('selections',
                            message=f"{colored('What sites do you want to upload too?', 'green')} {colored(f'{len(available)} available', 'yellow')}",
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
                        "file_name": "",
                        "file_url": "",
                        "exception": "",
                        "size_limit": "",
                        "extra": ""
                    }

                    uploader_classes = {
                        "Buzzheavier": Buzzheavier
                    }

                    if site in uploader_classes:
                        api_key = config.get("api_keys", {}).get(site, None) if sites_data_dict.get(site, "").get("apiKey") == True else None

                        site_instance = uploader_classes.get(site, "No_Site")(file, proxy_list, user_agents_list, api_key)
                        output = site_instance.Uploader()          


                        print(output)

                        # output = uploader_classes.get(site, "No_Site").Uploader(file, proxy_list, user_agents_list, api_key)

                        status = output.get("status", "404 status not found")
                        file_name = output.get("file_name", "oopsie_daisie.fish")
                        file_url = output.get("file_url", "url be doing the hidy hole")
                        exception_str = output.get("exception", "Fuck me there was no exception.")
                        size_limit = output.get("size_limit", "-3 GB")
                        extra = output.get("extra", "Monkey stole the bananas")
                        
                        os.chdir(owd)
                        if status == "ok":
                            print(f"{ok} {colored(file_name, 'light_blue')} {colored('successfully uploaded to', 'green')} {colored(site, 'yellow')}{colored('! URL:', 'green')} {colored(file_url, 'light_blue')}")
                            with open("file_links.txt", "a") as file_links:
                                file_links.writelines(f"{datetime.now()} | {site} | {file_name} - {file_url}\n")
                            file_links.close()
                            if auto_load_preset == True and link_format != "" and DEBUG == False:
                                with open("file_links_formatted.txt", "a") as formatted_links_file:
                                    formatted_links_file.writelines(f"{link_format.format(status=status, file_name=file_name, file_url=file_url, site_name=site, date_and_time=datetime.now())}\n")
                                formatted_links_file.close()
                        
                        elif status == "error":
                            if site in ["Transfer_sh", "Keep"]:
                                print(f"{error} {colored(site, 'yellow')} fucked up again while uploading the file {colored(file_name, 'light_blue')}. Don't Report this! Its a known issue they need to fix.")
                            else:
                                print(f"{error} An error occurred while uploading the file {colored(file_name, 'light_blue')} to {colored(site, 'yellow')}! Please report this. Exception: {colored(exception_str, 'red')}")
                                error_str = f"An error occurred while uploading the file {file_name} to {site}! Please report this. Exception: {exception_str}"
                                Logger.log_event(error_str, extra)
                            
                        elif status == "size_error":
                            print(f"{error} File size of {colored(file_name, 'light_blue')} to big for {colored(site, 'yellow')}! Compress it to fit the max size of {colored(size_limit, 'yellow')}")
                            error_str = f"File size of {file_name} to big for {site}! Compress it to fit the max size of {size_limit}"
                            Logger.log_event(error_str, extra)    
                        
                        else:
                            print(f"{major_error} An unknown error occured while uploading the file {colored(file_name, 'light_blue')} to {colored(site, 'yellow')}! Please report this. Exception: {colored(exception_str, 'red')}")
                            error_str = f"An unknown error occured while uploading the file {file_name} to {site}! Please report this. Exception: {exception_str}"
                            Logger.log_event(error_str, extra)
                    else:
                        touch_grass = False
                        feel_woman_touch = False
                        pass
                
                list_bar()      
            os.chdir(owd) # reset to default working dir