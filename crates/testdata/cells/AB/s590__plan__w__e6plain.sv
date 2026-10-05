module top;
  logic clk, en; wire gclk; int n;
  initial begin clk = 0; forever #5 clk = ~clk; end
  assign gclk = clk & en;
  always @(posedge gclk) n++;
  initial begin
    en = 1;
    #15; @(posedge clk) $display("S t=%0t n=%0d", $time, n);
    #30 $display("t=%0t n=%0d", $time, n);
    $finish;
  end
  initial #200 $finish;
endmodule
