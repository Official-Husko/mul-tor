from .site_data import sites_data_dict

final_sites = []

free_sites = []
paid_sites = []

class Availability_Checker:
    
    def Evaluate(config):
        for site in sites_data_dict:
            if site["apiKey"] == False:
                free_sites.append(site)
            else:
                paid_sites
        #TODO: ping all servers
        #TODO: Check all configured API's in config.json
        pass
    
    def ping():
        pass