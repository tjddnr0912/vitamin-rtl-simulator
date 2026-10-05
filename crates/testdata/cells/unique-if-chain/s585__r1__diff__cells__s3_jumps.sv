module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0;
  integer i;
  int q[3] = '{1, 2, 3};
  task automatic tr(input logic x, input logic y);
    unique if (x) return;
    else if (y) r = 2;
    $display("tr after t=%0t", $time);
  endtask
  function automatic void fr(input logic x, input logic y);
    unique if (x) return;
    else if (y) r = 2;
    $display("fr after t=%0t", $time);
  endfunction
  initial begin
    #1 for (i = 0; i < 3; i++) begin
      unique if (a) break;
      else if (b) continue;
      $display("for body i=%0d t=%0t", i, $time);
    end
    #1 for (i = 0; i < 3; i++) begin
      if (i == 1) continue;
      unique if (a) r = 1;
      else if (b) break;
    end
    #1 tr(a, b);
    #1 begin : blk
      unique if (a) disable blk;
      else if (b) r = 2;
      $display("blk after t=%0t", $time);
    end
    #1 i = 0; while (1) begin i++; unique if (i > 2) break; else if (a) r = 1; end
    #1 fr(a, b);
    #1 foreach (q[k]) begin unique if (q[k] == 2) continue; else if (a) break; $display("fe k=%0d", k); end
    #1 i = 0; do begin i++; unique if (i == 2) continue; else if (a) r = 1; end while (i < 3);
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
