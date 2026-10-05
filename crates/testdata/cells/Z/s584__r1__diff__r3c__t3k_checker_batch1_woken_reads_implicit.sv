module top;
  logic [1:0] cfg = 2'd1;
  logic mode;
  logic [1:0] p, y;
  initial mode = 1'b1;
  always_comb p = cfg;
  always_comb begin
    unique case ({mode, p})
      3'b101: y = 2'd1;
      3'b110: y = 2'd2;
      3'b001, 3'b010: y = 2'd0;
    endcase
  end
  initial #1 $display("t=%0t y=%0d", $time, y);
  initial #5 $finish;
endmodule
