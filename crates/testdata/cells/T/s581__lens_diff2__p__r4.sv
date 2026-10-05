`define GC(NM, LBL, TAG) case (-1) LBL: begin : NM wire [7:0] w = 8'd200; initial #1 $display("@%0d a %0d bits=%0d", TAG, w, $bits(w)); end default: begin : NM wire [3:0] w = 4'd9; initial #1 $display("@%0d def %0d bits=%0d", TAG, w, $bits(w)); end endcase
module top;
  localparam logic [31:0] KA = 32'hFFFFFFFF;
  if (1) begin : gb
    `GC(x1, KA, 1)
    `GC(x2, KB, 2)
    localparam logic [31:0] KB = 32'hFFFFFFFF;
  end
endmodule
