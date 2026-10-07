`timescale 1ns/1ps
module tbw; pcie_tlp_demux_bar #(.PORTS(2), .BAR_BASE(4), .BAR_STRIDE(7), .FIFO_DEPTH(64)) u(); initial begin #1 $display("IDS=%h", u.BAR_IDS_INT); $finish; end endmodule
