import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "CyberFile"

class CyberFile:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        req = "which one of you maggots ate the fucking request huh?"
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)
            
            chunk_size = 20000000  # 20 MB

            characters = string.ascii_lowercase + string.digits + string.ascii_uppercase
            filehosting = ''.join(random.choice(characters) for i in range(24))
            trial_username = ''.join(random.choice(characters) for i in range(32))
            trial_hash = ''.join(random.choice(characters) for i in range(32))

            cookies = {
                'filehosting': filehosting,
                'trial_username': f'trial_{trial_username}',
                'trial_hash': trial_hash,
                'jstree_select': '%23null'
            }

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        data = {
                            'maxChunkSize': chunk_size,
                            'uploadSource': 'file_manager',
                            '_sessionid': '43e292i2ienj2gsftsgvssv398',
                            'cTracker': 'd0cd6090a0758d137ee8ff0254f7c7ce'
                        }

                        form_data = {
                            'files[]': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                        }
                        
                        raw_req = requests.post(url=upload_url, data=data, files=form_data, cookies=cookies, headers=headers, proxies=proxies)
                
                print(raw_req.text)

                req = raw_req.json()
                download_url = req[0].get("url", "No_URL")

                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}