`timescale 1ns/1ps
module tbw; pcie_tlp_demux_bar #(.PORTS(6), .BAR_BASE(0), .BAR_STRIDE(1), .FIFO_DEPTH(64)) u(); initial begin #1 $display("IDS=%h", u.BAR_IDS_INT); $finish; end endmodule
