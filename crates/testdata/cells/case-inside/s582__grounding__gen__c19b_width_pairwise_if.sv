module top;
  logic [7:0] a, b; int m; logic [3:0] v;
  initial begin
    a = 8'h80; b = 8'h80; m = 9;
    if (a + b inside {8'h00}) m = 1;
    else if (a + b inside {16'h0100}) m = 2;
    else m = 0;
    $display("a+b m=%0d", m);
    a = 8'h01; b = 8'hFF; m = 9;
    if (a + b inside {8'h00}) m = 1;
    else if (a + b inside {16'h0100}) m = 2;
    else m = 0;
    $display("a+b m=%0d", m);
    a = 8'h00; b = 8'h00; m = 9;
    if (a + b inside {8'h00}) m = 1;
    else if (a + b inside {16'h0100}) m = 2;
    else m = 0;
    $display("a+b m=%0d", m);
    #10 $finish;
  end
endmodule
