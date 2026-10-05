module top;
  logic clk = 0; logic a, b, c; logic [1:0] q, r, r2; logic [1:0] s;
  always #5 clk = ~clk;
  always_ff @(posedge clk) begin
    unique if (a) q <= 1;
    else if (b) begin
      unique case (s) 2'd0: r <= 0; 2'd1: r <= 1; endcase
    end
    else if (c) q <= 3;
  end
  always @(posedge clk) begin
    unique case (s)
      2'd0: unique if (a) r2 <= 2; else if (b) r2 <= 3;
      2'd1: r2 <= 1;
      default: ;
    endcase
  end
  always @(negedge clk) $display("t=%0t q=%0d r=%0d r2=%0d", $time, q, r, r2);
  initial begin
    a = 0; b = 0; c = 0; s = 0; r2 = 0;
    #7 b = 1; s = 2;
    #10 b = 0; c = 1; s = 1;
    #10 c = 0; a = 1; s = 0;
    #10 fork
      unique if (a) $display("  f1 a t=%0t", $time); else if (b) $display("  f1 b");
      begin #1 priority if (a & b) $display("  f2"); else if (c) $display("  f2 c"); end
    join
    #3 $finish;
  end
  initial #100 $finish;
endmodule
