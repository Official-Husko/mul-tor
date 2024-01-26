import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "1Fichier"

class OneFichier:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "None :("
        try:
            ua = random.choice(user_agents)
            initialize_url = sites_data_dict[site]["initialize_url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
            
            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":

                raw_server = requests.get(url=initialize_url, headers={"User-Agent": ua, "Content-Type": "application/json"}, timeout=300)

                server_response = raw_server.json()
                server_url = server_response.get("url", "")
                file_id = server_response.get("id", "")

                upload_url = sites_data_dict[site]["url"].format(server=server_url, upload_id=file_id)

                form_data = {
                    'file[]': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, files=form_data, headers=headers, proxies=proxies, timeout=300, stream=True)
                
                finalize_url = sites_data_dict[site]["finalize_url"].format(server=server_url, upload_id=file_id)

                raw_req = requests.get(url=finalize_url, headers={"User-Agent": ua, "JSON": "1"}, proxies=proxies)

                raw_req = raw_req.json()
                download_url = raw_req.get("links", [])[0].get("download", "")

                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

