import json
import requests
import os
from time import sleep
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from .logger import Logger

site = "pixeldrain"

class PixelDrain:
    
    def Uploader(files, proxy_list, user_agents):
        for file in files:
            try:
                upload_url = sites_data_dict[site]["url"]
                size_limit = sites_data_dict[site]["size_limit_human"]
                size_unit = sites_data_dict[site]["size_unit"]
                base_url = sites_data_dict[site]["download_url_base"]
                
                file_size = os.stat(file).st_size
                file_name = file.rsplit("\\")
                file_name = file_name[-1]
                file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
                
                calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
                
                if calc_size == "OK":
                    with open(file, "rb") as file_upload:
                        ua = random.choice(user_agents)
                        if proxy_list == []:
                            req = requests.put(url=upload_url + file_name, data=file_upload, headers={"User-Agent": ua}).json()
                        else:
                            req = requests.put(url=upload_url + file_name, data=file_upload, headers={"User-Agent": ua}, proxies=random.choice(proxy_list)).json()
                        file_upload.close()
                        print(f"{ok} {colored(file_name, 'light_blue')} successfully uploaded! URL: {colored(base_url, 'green')}{colored(req['id'], 'green')}")
                        """with open("file_links.txt", "a") as file_links:
                            file_links.writelines(f"{file_name} - {base_url}{req['id']}")
                        file_links.close()""" # TODO: Fix this broken file. it saves to a wrong directory
                elif calc_size == "SIZE_ERROR":
                    print(f"{warning} File size to big for the service! Compress it to fit the max size of {colored(str(size_limit) + size_unit, 'yellow')}.")
                    sleep(5)
                    return
                else:
                    # Construct, log and print the error
                    error_str = f"An error occured during the Calculation of the file! Please report this."
                    print(f"{error} {error_str}")
                    Logger.log_event(error_str)
                    print(5)
                    return
                    
            except Exception as e:
                # Construct, log and print the error
                error_str = f"An error occured while uploading the file {file_name} to {site}! Please report this. Exception: {str(e)}"
                print(f"{error} {error_str}")
                Logger.log_event(error_str, req)
                sleep(5)
                return