module top;
  logic [1:0] y;
  task automatic rt(input int n, input logic a, input logic b);
    unique if (n == 0 && a) y = 1;
    else if (n == 0 && b) y = 2;
    else if (n > 0) begin rt(n - 1, a, b); end
  endtask
  task automatic tr(input logic a, input logic b);
    unique if (a) return; else if (b) y = 2;
    $display("  after-chain in tr t=%0t", $time);
  endtask
  initial begin
    #1 rt(2, 0, 0); $display("t=%0t rec miss", $time);
    #1 rt(2, 1, 0); $display("t=%0t rec hit", $time);
    #1 tr(1, 0); $display("t=%0t return hit", $time);
    #1 tr(0, 0); $display("t=%0t return miss", $time);
    #1 y = 3; begin : blk
      unique if (y == 3) disable blk; else if (y == 0) y = 1;
      $display("  after disable-chain y3 t=%0t", $time);
    end
    $display("t=%0t disable taken", $time);
    #1 y = 2; begin : blk2
      unique if (y == 3) disable blk2; else if (y == 0) y = 1;
      $display("  after disable-chain y2 t=%0t", $time);
    end
    $display("t=%0t disable miss", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
