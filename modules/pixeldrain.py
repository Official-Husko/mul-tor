import json
import requests
from .site_data import Site_Data_CLSS, sites_data_dict
import os
from .pretty_print import *
from time import sleep

site = "pixeldrain"

class PixelDrain:
    
    def Uploader(file, size):
        size_limit = sites_data_dict[site]["size_limit"]
        size_unit = sites_data_dict[site]["size_unit"]
        
        file_size = os.stat(file).st_size
        file_name = file.rsplit(("\\", "/"))
        file_name = (file_name[:255] + '..') if len(file_name) > 255 else file_name
        print(file_name)
        
        calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
        
        if size_limit <= calc_size:
            with open(file, "rb") as file_upload:
                #req = requests.put()
                file_upload.clear()
                pass
        else:
            print(f"[{error}] File size to big for the service! Compress it to fit the max size of {size_limit}{size_unit}.")
            sleep(5)
            return