import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from .logger import Logger

site = "PixelDrain"

class PixelDrain:
    
    def Uploader(file, proxy_list, user_agents):
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            base_url = sites_data_dict[site]["download_url_base"]
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
            
            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                with open(file, "rb") as file_upload:
                    req = requests.put(url=upload_url + file_name, data=file_upload, headers=headers, proxies=proxies).json()
                    file_upload.close()
                return {"status": "ok", "file_name": file_name, "file_url": base_url + req['id'], "site": site}
            else:
                return {"status": "size_error", "file_name": file_name, "site": site, "exception": "SIZE_ERROR", "size_limit": size_limit}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "site": site, "exception": str(e), "extra": req}