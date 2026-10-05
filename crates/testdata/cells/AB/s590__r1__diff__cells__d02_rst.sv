module top;
  logic clk = 0, rst_raw, rst_raw2;
  function automatic logic chk(input logic r);
    if (r === 1'bz) $display("never");
    chk = r;
  endfunction
  wire rst_n = chk(rst_raw);
  logic rst2_n;
  assign rst2_n = chk(rst_raw2);
  logic [3:0] q, q2;
  always @(posedge clk or negedge rst_n) if (!rst_n) q <= 0; else q <= q + 1;
  always @(posedge clk or negedge rst2_n) if (!rst2_n) q2 <= 0; else q2 <= q2 + 1;
  always @(negedge rst_n) $display("RN t=%0t", $time);
  always @(negedge rst2_n) $display("RN2 t=%0t", $time);
  initial begin rst_raw = 0; #12 rst_raw = 1; end
  initial begin #1 rst_raw2 = 0; #11 rst_raw2 = 1; end
  always #5 clk = ~clk;
  always @(posedge clk) #1 $display("t=%0t q=%0d q2=%0d", $time, q, q2);
  initial #33 $finish;
endmodule
