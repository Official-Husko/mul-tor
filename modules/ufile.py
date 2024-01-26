import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "uFile"

class uFile:
     def Uploader(file, proxy_list, user_agents, api_keys):
        """
        Uploads a file to a specified site using random user agents and proxies.

        Args:
            file (str): The path to the file to be uploaded.
            proxy_list (list): A list of proxy URLs.
            user_agents (list): A list of user agent strings.

        Returns:
            dict: A dictionary containing the status, file name, file URL, and site.

        Raises:
            Exception: If an error occurs during the upload process.
        """
        raw_req = "None :("
        try:
            # Select a random user agent
            ua = random.choice(user_agents)

            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Set the user agent header
            headers = {
                "User-Agent": ua,
                "Accept": "application/json"
            }

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            chunk_size = 69512747  # 66 MB
            chunk_index = 1
            total_chunks = (file_size + chunk_size - 1) // chunk_size

            if calc_size == "OK":

                server_url = sites_data_dict[site]["server_url"]

                raw_req = requests.post(url=server_url, headers=headers, proxies=proxies, timeout=300)

                raw_req = raw_req.json()

                storage_server = raw_req.get("storageBaseUrl", "")

                initialize_url = sites_data_dict[site]["initialize_url"].format(server=storage_server)

                upload_data = {
                    "file_size": file_size
                }

                raw_req = requests.post(url=initialize_url, data=upload_data, headers=headers, proxies=proxies, timeout=300)

                raw_req = raw_req.json()
                file_id = raw_req.get("fuid", "")

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        upload_data = {
                            "fuid": file_id,
                            "chunk_index": chunk_index
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'file': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Get the upload URL from the site data dictionary
                        upload_url = sites_data_dict[site]["url"].format(server=storage_server)

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=300)

                        chunk_index += 1

                finalize_url = sites_data_dict[site]["finalize_url"].format(server=storage_server)

                upload_data = {
                    "fuid": file_id,
                    "file_name": file_name,
                    "file_type": "unknown",
                    "total_chunks": total_chunks
                }

                raw_req = requests.post(url=finalize_url, data=upload_data, headers=headers, proxies=proxies, timeout=300)

                raw_req = raw_req.json()
                download_url = raw_req.get("url", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

