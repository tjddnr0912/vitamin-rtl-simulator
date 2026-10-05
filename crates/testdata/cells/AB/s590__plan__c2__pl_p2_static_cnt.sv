module top;
  logic a; logic [7:0] y;
  function automatic logic [7:0] f(input logic x);
    static int cnt = 0;
    cnt = cnt + 1;
    return cnt[7:0];
  endfunction
  assign y = f(a);
  initial begin
    a = 0;
    #1 $display("t=%0t y=%0d", $time, y);
    a = 1;
    #1 $display("t=%0t y=%0d", $time, y);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
