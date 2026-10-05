class K;
  int v;
  function int kf(input logic x, input logic z);
    unique if (x) return 1; else if (z) return 2;
    return 0;
  endfunction
  task kt(input logic x, input logic z);
    unique if (x) v = 1; else if (z) v = 2;
  endtask
endclass
module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0, y;
  function automatic int mf(input logic x);
    unique if (x) return 1; else if (!x) return 2;
  endfunction
  task automatic mt(input logic x, input logic z);
    unique if (x) r = 1; else if (z) r = 2;
  endtask
  always_comb begin
    y = 0;
    unique if (a) y = 1;
    else if (b) y = 2;
  end
  K k;
  initial begin : outer
    k = new;
    #1 begin : inner
      unique if (a) r = 1;
      else if (b) r = 2;
    end
    #1 void'(k.kf(a, b)); k.kt(a, b); mt(a, b);
    #1 a = 1; #1 a = 0;
    #1 $finish;
  end
endmodule
