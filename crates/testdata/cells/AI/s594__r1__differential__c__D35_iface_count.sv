interface ifc #(parameter N = 4);
  localparam L = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 128);
endinterface
module t;
  ifc #(.N(16)) i();
  initial $display("R: L=%0d", i.L);
  initial #10 $finish;
endmodule
