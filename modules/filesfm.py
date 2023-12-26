import requests
import os
import random
from time import sleep

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "FilesFM"

class FilesFM:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            initialize_url = sites_data_dict[site]["initialize_url"]
            download_url_base = sites_data_dict[site]["download_url_base"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            raw_req = requests.get(url=initialize_url, headers=headers, proxies=proxies)

            server_response = str(raw_req.text).split(",")

            upload_id = server_response[0]
            upload_key = server_response[2]

            upload_url = sites_data_dict[site]["url"].format(upload_id=upload_id, upload_key=upload_key)

            if calc_size == "OK":
                form_data = {
                    'Filedata': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }

                raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies, stream=True)

                finalize_url = sites_data_dict[site]["finalize_url"].format(upload_id=upload_id)
                fin_req = requests.get(url=finalize_url, headers=headers, proxies=proxies)

                result = fin_req.json()

                if result.get("status", "") == "ok":
                    return {"status": "ok", "file_name": file_name, "file_url": download_url_base.format(upload_id=upload_id)}
                else:
                    raise Exception("Upload Failed :(")
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": req}

