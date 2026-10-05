`define GC(T, L) case (-1) 32'hFFFFFFFF: begin wire [7:0] w = 8'd200; initial #1 $display("%s a %0d bits=%0d", T, w, $bits(w)); end L: begin wire [7:0] w = 8'd2; initial #1 $display("%s k %0d", T, w); end default: begin wire [3:0] w = 4'd9; initial #1 $display("%s def %0d bits=%0d", T, w, $bits(w)); end endcase
module M1;
  if (1) begin : gb
    `GC("first", 0)
    `GC("second", K)
    localparam logic [7:0] K = 8'd99;
  end
  initial #5 $finish;
endmodule
