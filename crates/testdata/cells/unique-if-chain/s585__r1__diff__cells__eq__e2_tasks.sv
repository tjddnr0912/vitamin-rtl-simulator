module top;
  logic clk = 0;
  logic [2:0] c = 0;
  logic [3:0] o1, o2, acc = 0;
  always #5 clk = ~clk;
  task automatic sel(input logic [2:0] k, output logic [3:0] r);
    unique if (k < 3) r = k + 1;
    else if (k < 6) r = k + 2;
    else if (k >= 6) r = 4'hE;
  endtask
  task automatic accum(ref logic [3:0] a, input logic [2:0] k);
    priority if (k[0]) a = a + 1;
    else if (k[1]) a = a + 2;
    else if (k[2]) a = a + 4;
  endtask
  always_ff @(posedge clk) begin c <= c + 1; accum(acc, c); end
  always_comb sel(c, o1);
  always @(negedge clk) sel(c ^ 3'b101, o2);
  initial begin
    $dumpfile("e2.vcd"); $dumpvars(0, top);
    $monitor("t=%0t c=%0d o1=%h o2=%h acc=%h", $time, c, o1, o2, acc);
    #120 $finish;
  end
endmodule
