#include "osm_consumer_example_impl.h"

#include "logos_sdk.h"

std::string OsmConsumerExampleImpl::resolveRegion(const std::string& region)
{
    return modules().osm.resolve(region);
}

std::string OsmConsumerExampleImpl::downloadRegion(const std::string& region,
                                                   const std::string& destDir)
{
    return modules().osm.downloadRegion(region, destDir);
}
