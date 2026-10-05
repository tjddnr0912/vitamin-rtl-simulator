module top;
  logic a = 0, b = 0, c = 0;
  logic [1:0] r = 0;
  int n = 0;
  function automatic logic inc(input logic v); n = n + 1; return v; endfunction
  initial begin
    #1 unique if (a) r = 1;
       else if (b) r = 2;
       else if (c) r = 3;
    $display("t=%0t multiline done", $time);
    #1 unique if (a) r = 1;
       else unique if (b) r = 2;
    $display("t=%0t split else-unique-if done", $time);
    #1 unique if (a) r = 1;
       else unique0 if (b) r = 2;
    $display("t=%0t split else-unique0-if done", $time);
    #1 unique if (a) r = 1;
       else unique if (b) r = 2;
       else if (c) r = 3;
    $display("t=%0t split middle-unique chain3 done", $time);
    #1 unique if (a) r = 1;
       else unique0 if (b) r = 2;
       else if (c) r = 3;
    $display("t=%0t split middle-unique0 chain3 done", $time);
    #1 unique if (a) r = 1; else if (b) r = 2; else ;
    $display("t=%0t explicit null else done", $time);
    #1 n = 0; unique if (inc(a)) r = 1; else if (inc(b)) r = 2;
    $display("t=%0t side-effect n=%0d", $time, n);
    #1 n = 0; b = 1; unique if (inc(a)) r = 1; else if (inc(b)) r = 2; else if (inc(c)) r = 3;
    $display("t=%0t side-effect match n=%0d r=%0d", $time, n, r);
    #1 a = 1; b = 0; unique if (a) begin if (c) r = 1; end else if (b) r = 2;
    $display("t=%0t matched-then inner-miss done", $time);
    #1 $finish;
  end
endmodule
