local fennel, api = ...

-- load fennel
package.preload.fennel = fennel

-- add the scripts directory to the load path
package.path = './scripts/?.lua;' .. package.path

-- TODO placeholder functions for all the other stuff like file emitting and such

